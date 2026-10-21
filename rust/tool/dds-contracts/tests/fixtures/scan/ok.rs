// A sanctioned Node API call: must not be flagged by the scan.
fn demo(node: &Node) {
    let _writer = node.writer::<HelloWorldTopic>("HelloWorldTopic");
}
