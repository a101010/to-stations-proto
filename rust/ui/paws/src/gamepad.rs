//! Controller enumeration and selection.

use std::rc::Rc;

use gilrs::{GamepadId, Gilrs};
use slint::{SharedString, VecModel};

use crate::state::SavedController;

/// The stable part of a controller identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChoiceKey {
    pub(crate) name: String,
    pub(crate) uuid: [u8; 16],
    pub(crate) id: usize,
}

/// One connected controller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Choice {
    pub(crate) id: GamepadId,
    pub(crate) key: ChoiceKey,
}

impl Choice {
    pub(crate) fn saved(&self) -> SavedController {
        SavedController {
            name: self.key.name.clone(),
            uuid: self.key.uuid,
            id: self.key.id,
        }
    }

    pub(crate) fn label(&self) -> String {
        label(&self.key.name, self.key.id)
    }
}

/// The current controller choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Selection {
    Virtual,
    Gamepad(GamepadId),
}

/// List connected controllers that use an Xbox-style name.
pub(crate) fn xbox_choices(gilrs: &Gilrs) -> Vec<Choice> {
    gilrs
        .gamepads()
        .filter(|(_, gamepad)| is_xbox_style(gamepad.name()))
        .map(|(id, gamepad)| Choice {
            id,
            key: ChoiceKey {
                name: gamepad.name().to_string(),
                uuid: gamepad.uuid(),
                id: usize::from(id),
            },
        })
        .collect()
}

/// Whether a controller name identifies an Xbox or XInput device.
pub(crate) fn is_xbox_style(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.contains("xbox") || name.contains("xinput")
}

/// Replace the model with `Virtual` followed by the controller labels.
pub(crate) fn update_model(choices: &[Choice], model: &Rc<VecModel<SharedString>>) {
    let mut labels: Vec<SharedString> = Vec::with_capacity(choices.len() + 1);
    labels.push("Virtual".into());
    labels.extend(choices.iter().map(|choice| choice.label().into()));
    model.set_vec(labels);
}

/// Refresh the controller list and its dropdown model.
///
/// Returns whether the controller list changed.
pub(crate) fn refresh_choices(
    gilrs: &Gilrs,
    choices: &mut Vec<Choice>,
    model: &Rc<VecModel<SharedString>>,
) -> bool {
    let fresh = xbox_choices(gilrs);
    if *choices == fresh {
        return false;
    }
    *choices = fresh;
    update_model(choices, model);
    true
}

/// The copyable choice at dropdown index `index`; `None` means `Virtual`.
#[cfg(test)]
pub(crate) fn choice_for_index<T: Copy>(index: i32, choices: &[T]) -> Option<T> {
    if index <= 0 {
        None
    } else {
        choices.get((index - 1) as usize).copied()
    }
}

/// Map a dropdown index to a selection.
pub(crate) fn selection_for_index(index: i32, choices: &[Choice]) -> Selection {
    if index <= 0 {
        Selection::Virtual
    } else {
        choices
            .get((index - 1) as usize)
            .map(|choice| Selection::Gamepad(choice.id))
            .unwrap_or(Selection::Virtual)
    }
}

/// The dropdown index for `selection`.
pub(crate) fn index_for_selection(selection: Selection, choices: &[Choice]) -> i32 {
    match selection {
        Selection::Virtual => 0,
        Selection::Gamepad(id) => choices
            .iter()
            .position(|choice| choice.id == id)
            .map(|index| index as i32 + 1)
            .unwrap_or(0),
    }
}

/// The remembered controller's connected id, if present.
pub(crate) fn find_remembered(choices: &[Choice], saved: &SavedController) -> Option<GamepadId> {
    let keys: Vec<ChoiceKey> = choices.iter().map(|choice| choice.key.clone()).collect();
    remembered_index(&keys, saved).map(|index| choices[index].id)
}

/// The index of the remembered controller, preferring the saved gilrs id.
///
/// An all-zero UUID is treated as unknown, because some Windows drivers report
/// it for every Xbox controller.
pub(crate) fn remembered_index(keys: &[ChoiceKey], saved: &SavedController) -> Option<usize> {
    let zero_uuid = saved.uuid.iter().all(|byte| *byte == 0);

    let mut matching: Vec<usize> = if zero_uuid {
        keys.iter()
            .enumerate()
            .filter(|(_, key)| key.name == saved.name)
            .map(|(index, _)| index)
            .collect()
    } else {
        keys.iter()
            .enumerate()
            .filter(|(_, key)| key.name == saved.name && key.uuid == saved.uuid)
            .map(|(index, _)| index)
            .collect()
    };

    if matching.is_empty() && zero_uuid {
        // The driver name can change between backends (for example WGI vs
        // XInput). With no UUID, the saved id is the remaining identity.
        matching = (0..keys.len()).collect();
    } else if matching.is_empty() {
        // Allow a unique UUID match when the reported name changed.
        matching = keys
            .iter()
            .enumerate()
            .filter(|(_, key)| key.uuid == saved.uuid)
            .map(|(index, _)| index)
            .collect();
    }

    choose_candidate(keys, &matching, saved.id)
}

/// Prefer the saved id; otherwise only accept an unambiguous match.
fn choose_candidate(keys: &[ChoiceKey], matching: &[usize], saved_id: usize) -> Option<usize> {
    if let Some(index) = matching
        .iter()
        .copied()
        .find(|&index| keys[index].id == saved_id)
    {
        return Some(index);
    }
    if matching.len() == 1 {
        Some(matching[0])
    } else {
        None
    }
}

/// A dropdown label for a controller.
pub(crate) fn label(name: &str, id: usize) -> String {
    format!("{name} ({id})")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(name: &str, uuid: u8, id: usize) -> ChoiceKey {
        ChoiceKey {
            name: name.to_string(),
            uuid: [uuid; 16],
            id,
        }
    }

    fn saved(name: &str, uuid: u8, id: usize) -> SavedController {
        SavedController {
            name: name.to_string(),
            uuid: [uuid; 16],
            id,
        }
    }

    #[test]
    fn xbox_names_are_detected_case_insensitively() {
        assert!(is_xbox_style("Xbox Wireless Controller"));
        assert!(is_xbox_style("XInput Controller #1"));
        assert!(!is_xbox_style("Saitek X52"));
        assert!(!is_xbox_style("DualSense Wireless Controller"));
    }

    #[test]
    fn choice_for_index_skips_virtual() {
        let choices = [10, 20];
        assert_eq!(choice_for_index(0, &choices), None);
        assert_eq!(choice_for_index(1, &choices), Some(10));
        assert_eq!(choice_for_index(2, &choices), Some(20));
        assert_eq!(choice_for_index(3, &choices), None);
    }

    #[test]
    fn remembered_index_prefers_saved_id() {
        let keys = [key("Xbox Controller", 7, 1), key("Xbox Controller", 7, 3)];
        assert_eq!(
            remembered_index(&keys, &saved("Xbox Controller", 7, 3)),
            Some(1)
        );
        assert_eq!(
            remembered_index(&keys, &saved("Xbox Controller", 7, 9)),
            None
        );
    }

    #[test]
    fn remembered_index_accepts_a_unique_uuid_with_a_changed_name() {
        let keys = [key("Xbox Controller", 7, 0)];
        assert_eq!(remembered_index(&keys, &saved("Other", 7, 0)), Some(0));
        assert_eq!(
            remembered_index(&keys, &saved("Xbox Controller", 8, 0)),
            None
        );
    }

    #[test]
    fn zero_uuid_falls_back_to_the_only_controller() {
        let keys = [key("Xbox Controller", 0, 0)];
        assert_eq!(
            remembered_index(&keys, &saved("Xbox One Game Controller", 0, 0)),
            Some(0)
        );
    }

    #[test]
    fn zero_uuid_uses_the_saved_id_when_the_name_changes() {
        let keys = [key("Xbox Controller", 0, 0), key("Xbox Controller", 0, 1)];
        assert_eq!(
            remembered_index(&keys, &saved("Xbox One Game Controller", 0, 1)),
            Some(1)
        );
        assert_eq!(
            remembered_index(&keys, &saved("Xbox One Game Controller", 0, 9)),
            None
        );
    }

    #[test]
    fn zero_uuid_prefers_the_saved_id_but_rejects_ambiguity() {
        let keys = [
            key("Xbox One Game Controller", 0, 0),
            key("Xbox One Game Controller", 0, 1),
        ];
        assert_eq!(
            remembered_index(&keys, &saved("Xbox One Game Controller", 0, 1)),
            Some(1)
        );
        assert_eq!(
            remembered_index(&keys, &saved("Xbox One Game Controller", 0, 9)),
            None
        );
    }

    #[test]
    fn label_includes_id() {
        assert_eq!(label("Xbox Controller", 2), "Xbox Controller (2)");
    }
}
