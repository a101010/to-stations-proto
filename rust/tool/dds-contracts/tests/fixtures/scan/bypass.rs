// A raw CycloneDDS topic-creation call: must be flagged by the scan.
fn demo(participant: &DomainParticipant) {
    let _topic = participant.create_topic::<HelloWorldTopic>("HelloWorldTopic");
}
