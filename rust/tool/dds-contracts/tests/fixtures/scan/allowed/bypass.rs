// A raw call in an allowed subtree: skipped when `--allow` covers this path.
fn demo(participant: &DomainParticipant) {
    let _topic = participant.create_topic::<HelloWorldTopic>("HelloWorldTopic");
}
