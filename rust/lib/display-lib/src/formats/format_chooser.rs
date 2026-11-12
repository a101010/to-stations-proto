//! The format chooser: a 4x4 grid of the other formats' icons and titles.
//!
//! The chooser is a normal format. Selecting a tile publishes a logical
//! `Select` event through the format manager; clicking anywhere else in the
//! region is consumed.

use std::path::Path;

use glow::HasContext as _;

use crate::formats::{Format, FormatInput, FormatMeta, FORMAT_CHOOSER};
use crate::icon::{Icon, IconRenderer, ICON_SIZE};
use crate::text::{TextRenderer, FONT_FILE};
use crate::ui::{self, Rect};
use crate::DisplayError;

/// Grid columns per page.
pub(crate) const GRID_COLUMNS: usize = 4;
/// Grid rows per page.
pub(crate) const GRID_ROWS: usize = 4;
/// Formats shown per page.
pub(crate) const PAGE_CAPACITY: usize = GRID_COLUMNS * GRID_ROWS;

/// Chooser background.
const BACKGROUND: [f32; 4] = [0.02, 0.03, 0.02, 1.0];
/// Tile background.
const TILE_BG: [f32; 4] = [0.06, 0.10, 0.07, 1.0];
/// Tile title color.
const TITLE_COLOR: [f32; 4] = [0.80, 1.0, 0.85, 1.0];

/// One chooser entry.
struct Entry {
    name: String,
    title: &'static str,
    icon: Icon,
}

/// One tile's geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ChooserTile {
    pub(crate) rect: Rect,
    pub(crate) icon: Rect,
    pub(crate) title: Rect,
    pub(crate) index: usize,
}

/// The chooser's geometry for the last region.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChooserLayout {
    pub(crate) tiles: Vec<ChooserTile>,
    /// Reserved space for future next/previous page controls.
    pub(crate) pager: Rect,
}

impl ChooserLayout {
    pub(crate) fn new(region: Rect, count: usize) -> Self {
        if region.w <= 0 || region.h <= 0 || count == 0 {
            return Self {
                tiles: Vec::new(),
                pager: Rect::new(region.x, region.y, 0, 0),
            };
        }

        let scale = ui::scale_for_height(region.h);
        let margin = (12.0 * scale).round() as i32;
        let gap = (8.0 * scale).round() as i32;
        let pager_height = (48.0 * scale).round() as i32;
        let tile_padding = (6.0 * scale).round() as i32;

        let content_x = region.x + margin;
        let content_y = region.y + margin;
        let content_w = (region.w - 2 * margin).max(1);
        let content_h = (region.h - 2 * margin - pager_height - gap).max(1);

        let cell_w = ((content_w - (GRID_COLUMNS as i32 - 1) * gap).max(1)) / GRID_COLUMNS as i32;
        let cell_h = ((content_h - (GRID_ROWS as i32 - 1) * gap).max(1)) / GRID_ROWS as i32;

        let pager = Rect::new(
            content_x,
            region.y + region.h - margin - pager_height,
            content_w,
            pager_height,
        );

        let visible = count.min(PAGE_CAPACITY);
        let mut tiles = Vec::with_capacity(visible);
        for index in 0..visible {
            let column = (index % GRID_COLUMNS) as i32;
            let row = (index / GRID_COLUMNS) as i32;
            let rect = Rect::new(
                content_x + column * (cell_w + gap),
                content_y + row * (cell_h + gap),
                cell_w,
                cell_h,
            );

            let inner = Rect::new(
                rect.x + tile_padding,
                rect.y + tile_padding,
                (rect.w - 2 * tile_padding).max(1),
                (rect.h - 2 * tile_padding).max(1),
            );
            let title_h = ((inner.h as f32 * 0.28).round() as i32).clamp(0, inner.h);
            let icon_h = (inner.h - title_h).max(1);
            let icon_area = Rect::new(inner.x, inner.y, inner.w, icon_h);
            let size = ((ICON_SIZE * scale).round() as i32)
                .min(icon_area.w)
                .min(icon_area.h)
                .max(1);
            let icon = Rect::new(
                icon_area.x + (icon_area.w - size) / 2,
                icon_area.y + (icon_area.h - size) / 2,
                size,
                size,
            );
            let title = Rect::new(inner.x, inner.y + icon_h, inner.w, title_h);

            tiles.push(ChooserTile {
                rect,
                icon,
                title,
                index,
            });
        }

        Self { tiles, pager }
    }

    /// The page index of the tile under `(x, y)`.
    pub(crate) fn tile_at(&self, x: i32, y: i32) -> Option<usize> {
        self.tiles
            .iter()
            .find(|tile| tile.rect.contains(x, y))
            .map(|tile| tile.index)
    }
}

/// The formats the chooser offers: every catalog entry except itself.
fn selectable(catalog: &[FormatMeta]) -> Vec<&FormatMeta> {
    catalog
        .iter()
        .filter(|meta| meta.name != FORMAT_CHOOSER)
        .collect()
}

/// The format chooser.
pub(crate) struct FormatChooser {
    entries: Vec<Entry>,
    icons: IconRenderer,
    text: TextRenderer,
}

impl FormatChooser {
    pub(crate) fn new(
        gl: &glow::Context,
        assets_root: &Path,
        catalog: &[FormatMeta],
    ) -> Result<Self, DisplayError> {
        let mut entries = Vec::new();
        for meta in selectable(catalog) {
            let icon = Icon::load(gl, &assets_root.join(meta.icon_file))?;
            entries.push(Entry {
                name: meta.name.clone(),
                title: meta.title,
                icon,
            });
        }
        let icons = IconRenderer::new(gl)?;
        let text = TextRenderer::new(gl, &assets_root.join(FONT_FILE))?;
        Ok(Self {
            entries,
            icons,
            text,
        })
    }
}

impl Format for FormatChooser {
    fn name(&self) -> &'static str {
        FORMAT_CHOOSER
    }

    fn title(&self) -> &'static str {
        "Formats"
    }

    fn icon_file(&self) -> &'static str {
        ""
    }

    fn render(
        &mut self,
        gl: &glow::Context,
        framebuffer_height: i32,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        if width <= 0 || height <= 0 {
            return;
        }
        let region = Rect::new(x, y, width, height);
        ui::clear_rect(gl, framebuffer_height, region, BACKGROUND);

        let y_gl = framebuffer_height - y - height;
        unsafe {
            gl.viewport(x, y_gl, width, height);
            gl.disable(glow::DEPTH_TEST);
        }

        let scale = ui::scale_for_height(height);
        let layout = ChooserLayout::new(region, self.entries.len());
        for tile in &layout.tiles {
            ui::clear_rect(gl, framebuffer_height, tile.rect, TILE_BG);
        }
        for tile in &layout.tiles {
            let entry = &self.entries[tile.index];
            let icon_rect = entry.icon.fit(tile.icon);
            self.icons.draw(gl, region, icon_rect, &entry.icon);

            let max_size = (18.0 * scale).min(tile.title.h as f32 * 0.8).max(1.0);
            let font_size = self
                .text
                .font_size_for(entry.title, tile.title.w as f32, max_size);
            let text_width = self.text.text_width(entry.title, font_size);
            let left = ((tile.title.w as f32 - text_width) / 2.0).max(0.0);
            let top = ((tile.title.h as f32 - font_size) / 2.0).max(0.0);
            self.text.draw_at(
                gl,
                width,
                height,
                entry.title,
                TITLE_COLOR,
                font_size,
                (tile.title.x - region.x) as f32 + left,
                (tile.title.y - region.y) as f32 + top,
            );
        }
    }

    fn mouse_button(&mut self, region: (i32, i32, i32, i32), x: i32, y: i32) -> FormatInput {
        let region = Rect::new(region.0, region.1, region.2, region.3);
        if !region.contains(x, y) {
            return FormatInput::Ignored;
        }
        let layout = ChooserLayout::new(region, self.entries.len());
        match layout.tile_at(x, y) {
            Some(index) => FormatInput::Select(self.entries[index].name.clone()),
            None => FormatInput::Consumed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(name: &str) -> FormatMeta {
        FormatMeta {
            name: name.to_string(),
            title: "Title",
            icon_file: "icons/default.png",
        }
    }

    fn center(rect: Rect) -> (i32, i32) {
        (rect.x + rect.w / 2, rect.y + rect.h / 2)
    }

    #[test]
    fn selectable_excludes_chooser_and_preserves_order() {
        let catalog = vec![meta("hello-world"), meta(FORMAT_CHOOSER), meta("default")];
        let names: Vec<&str> = selectable(&catalog)
            .iter()
            .map(|meta| meta.name.as_str())
            .collect();
        assert_eq!(names, ["hello-world", "default"]);
    }

    #[test]
    fn layout_has_sixteen_cells_in_four_columns() {
        let layout = ChooserLayout::new(Rect::new(0, 0, 720, 720), 16);
        assert_eq!(layout.tiles.len(), 16);
        for row in 0..GRID_ROWS {
            for column in 0..GRID_COLUMNS {
                let tile = layout.tiles[row * GRID_COLUMNS + column];
                assert_eq!(tile.index, row * GRID_COLUMNS + column);
                assert_eq!(tile.rect.y, layout.tiles[row * GRID_COLUMNS].rect.y);
            }
        }
        assert!(layout.tiles[GRID_COLUMNS].rect.y > layout.tiles[0].rect.y);
    }

    #[test]
    fn layout_stays_inside_region_and_above_pager() {
        let region = Rect::new(0, 0, 720, 720);
        let layout = ChooserLayout::new(region, 16);
        assert!(layout.pager.w > 0 && layout.pager.h > 0);
        for tile in &layout.tiles {
            assert!(tile.rect.x >= region.x);
            assert!(tile.rect.y >= region.y);
            assert!(tile.rect.x + tile.rect.w <= region.x + region.w);
            assert!(tile.rect.y + tile.rect.h <= region.y + region.h);
            assert!(tile.rect.y + tile.rect.h <= layout.pager.y);
            assert!(tile.icon.w > 0 && tile.icon.h > 0);
            assert!(tile.title.h >= 0);
        }
    }

    #[test]
    fn two_entries_use_only_the_first_two_cells() {
        let layout = ChooserLayout::new(Rect::new(0, 0, 720, 720), 2);
        assert_eq!(layout.tiles.len(), 2);
        assert_eq!(layout.tiles[0].index, 0);
        assert_eq!(layout.tiles[1].index, 1);
    }

    #[test]
    fn tile_at_maps_centers_and_ignores_empty_area() {
        let layout = ChooserLayout::new(Rect::new(0, 0, 720, 720), 2);
        let (x, y) = center(layout.tiles[1].rect);
        assert_eq!(layout.tile_at(x, y), Some(1));
        assert_eq!(layout.tile_at(719, 719), None);
    }

    #[test]
    fn empty_catalog_has_no_tiles() {
        let layout = ChooserLayout::new(Rect::new(0, 0, 720, 720), 0);
        assert!(layout.tiles.is_empty());
        assert_eq!(layout.tile_at(10, 10), None);
    }

    #[test]
    fn invalid_region_has_no_tiles() {
        let layout = ChooserLayout::new(Rect::new(0, 0, 0, 720), 2);
        assert!(layout.tiles.is_empty());
    }
}
