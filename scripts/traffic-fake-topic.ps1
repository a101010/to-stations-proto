# Sends crafted DDSI-RTPS datagrams to the StationsDDS multicast group so dds-echo
# can be verified without a full CycloneDDS participant. The datagrams are
# representative: several submessages each, covering every named submessage kind
# (PAD, ACKNACK, HEARTBEAT, GAP, INFO_TS, INFO_SRC, INFO_DST, DATA, DATA_FRAG)
# plus an unknown id (shown as 0xNN); SEDP endpoint announcements for two
# unregistered topics (FakeTopicOne, FakeTopicTwo) and a reader; an SPDP
# participant announcement; and user data for both topics so they appear under
# the "Unknown" group with packet counts.
#
# This exercises dds-echo's parsing/attribution/UI path; it is not a substitute
# for capturing real CycloneDDS SEDP.

[CmdletBinding()]
param(
    [string]$Group = 'ff02::ffff:239.255.0.1',
    [int]$Port = 7401,
    [int]$Repeat = 1,
    [int]$DelayMs = 200
)

$ErrorActionPreference = 'Stop'

$addr = Get-NetIPAddress -InterfaceAlias 'StationsDDS' -AddressFamily IPv6 -ErrorAction SilentlyContinue |
    Where-Object { $_.AddressState -eq 'Preferred' -or $_.AddressState -eq 'Tentative' } |
    Select-Object -First 1
if (-not $addr) {
    throw "No IPv6 address on the StationsDDS adapter. Run scripts/verify-loopback.ps1."
}
$ifindex = $addr.InterfaceIndex

function U16LE([int]$v) { [byte[]]@(($v -band 0xff), (($v -shr 8) -band 0xff)) }
function U32LE([int]$v) {
    [byte[]]@(($v -band 0xff), (($v -shr 8) -band 0xff), (($v -shr 16) -band 0xff), (($v -shr 24) -band 0xff))
}
function U32BE([int]$v) {
    [byte[]]@((($v -shr 24) -band 0xff), (($v -shr 16) -band 0xff), (($v -shr 8) -band 0xff), ($v -band 0xff))
}

# Concatenate byte[]s into one byte[].
function Concat([object[]]$parts) {
    $out = [System.Collections.Generic.List[byte]]::new()
    foreach ($p in $parts) { $out.AddRange([byte[]]$p) }
    return , $out.ToArray()
}

function B0() { return , ([byte[]]@()) }

# A submessage: id, flags, octetsToNextHeader, body.
function Sub([int]$id, [int]$flags, [byte[]]$body) {
    return Concat @(([byte[]]@($id, $flags)), (U16LE $body.Length), $body)
}

# A PL_CDR_LE parameter: pid/len little-endian, value padded to a multiple of 4.
function Param([int]$id, [byte[]]$value) {
    $padded = ($value.Length + 3) -band (-bnot 3)
    $out = [System.Collections.Generic.List[byte]]::new()
    $out.AddRange([byte[]](U16LE $id))
    $out.AddRange([byte[]](U16LE $padded))
    $out.AddRange($value)
    for ($i = $value.Length; $i -lt $padded; $i++) { $out.Add(0) }
    return , $out.ToArray()
}

# A CDR string: u32 length (including the NUL) then the bytes and NUL.
function CdrString([string]$s) {
    $bytes = [System.Text.Encoding]::ASCII.GetBytes($s)
    return Concat @((U32LE ($bytes.Length + 1)), $bytes, ([byte[]]@(0)))
}

function RtpsHeader([byte[]]$prefix) {
    return Concat @(
        ([System.Text.Encoding]::ASCII.GetBytes('RTPS')),
        ([byte[]]@(2, 4)),        # protocol version
        ([byte[]]@(0x01, 0x0f)),  # vendor id (Eclipse)
        $prefix)
}

function InfoTs() { return Sub 0x09 0x01 ([byte[]]@(0, 0, 0, 0, 0, 0, 0, 0)) }
function InfoSrc([byte[]]$prefix) {
    return Sub 0x0c 0x01 (Concat @(([byte[]]@(0, 0, 0, 0)), ([byte[]]@(2, 4)), ([byte[]]@(0x01, 0x0f)), $prefix))
}
function InfoDst([byte[]]$prefix) { return Sub 0x0e 0x01 $prefix }
function Pad() { return Sub 0x01 0x01 ([byte[]]@(0, 0, 0, 0)) }

function DataSub([int]$writer, [int]$sn, [byte[]]$payload) {
    $body = Concat @(
        ([byte[]]@(0, 0)),        # extraFlags
        ([byte[]]@(16, 0)),       # octetsToInlineQos
        ([byte[]]@(0, 0, 0, 0)),  # reader id
        (U32BE $writer),
        (U32LE 0), (U32LE $sn))
    return Sub 0x15 0x05 (Concat @($body, $payload))
}

function DataFrag([int]$writer, [int]$sn) {
    $body = Concat @(
        ([byte[]]@(0, 0)), ([byte[]]@(16, 0)), ([byte[]]@(0, 0, 0, 0)),
        (U32BE $writer), (U32LE 0), (U32LE $sn),
        (U32LE 1), ([byte[]]@(1, 0)), ([byte[]]@(0, 4)),
        (U32LE 1024))
    return Sub 0x16 0x05 $body
}

function Heartbeat([int]$writer, [int]$first, [int]$last, [int]$count) {
    return Sub 0x07 0x01 (Concat @(
        ([byte[]]@(0, 0, 0, 0)), (U32BE $writer),
        (U32LE 0), (U32LE $first), (U32LE 0), (U32LE $last), (U32LE $count)))
}

function AckNack([int]$writer, [int]$base) {
    return Sub 0x06 0x01 (Concat @(
        ([byte[]]@(0, 0, 0, 0)), (U32BE $writer),
        (U32LE 0), (U32LE $base),
        (U32LE 1), (U32LE 0),
        (U32LE 1)))
}

function Gap([int]$writer, [int]$start) {
    return Sub 0x08 0x01 (Concat @(
        ([byte[]]@(0, 0, 0, 0)), (U32BE $writer),
        (U32LE 0), (U32LE $start),
        (U32LE 1), (U32LE 0)))
}

# An SEDP endpoint announcement payload: ENDPOINT_GUID + TOPIC_NAME + TYPE_NAME.
function EndpointPayload([byte[]]$prefix, [int]$entity, [string]$topic, [string]$type) {
    $guid = Concat @($prefix, (U32BE $entity))
    return Concat @(
        ([byte[]]@(0x00, 0x03, 0x00, 0x00)),          # PL_CDR_LE
        (Param 0x005a $guid),                          # PID_ENDPOINT_GUID
        (Param 0x0005 (CdrString $topic)),             # PID_TOPIC_NAME
        (Param 0x0007 (CdrString $type)),              # PID_TYPE_NAME
        (Param 0x0001 (B0)))                           # PID_SENTINEL
}

# An SPDP participant announcement payload: PARTICIPANT_GUID + sentinel.
function ParticipantPayload([byte[]]$prefix) {
    $guid = Concat @($prefix, (U32BE 0x000001c1))      # ENTITYID_PARTICIPANT
    return Concat @(
        ([byte[]]@(0x00, 0x03, 0x00, 0x00)),
        (Param 0x0050 $guid),                          # PID_PARTICIPANT_GUID
        (Param 0x0001 (B0)))
}

# A CDR_LE-encoded hello_world::HelloWorldTopic: string greeting + Quaternion.
function CdrHelloWorldTopic([string]$greeting, [double]$x, [double]$y, [double]$z, [double]$w) {
    $s = [System.Text.Encoding]::ASCII.GetBytes($greeting)
    $out = [System.Collections.Generic.List[byte]]::new()
    $out.AddRange([byte[]]@(0x00, 0x01, 0x00, 0x00))  # CDR_LE encapsulation
    $out.AddRange([byte[]](U32LE ($s.Length + 1)))     # string length incl NUL
    $out.AddRange($s)
    $out.Add(0)
    while ((($out.Count - 4) % 8) -ne 0) { $out.Add(0) }  # align body to the doubles
    foreach ($d in @($x, $y, $z, $w)) {
        $out.AddRange([byte[]][BitConverter]::GetBytes([double]$d))
    }
    return , $out.ToArray()
}

$prefix = [byte[]]@(0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c)
$w1 = 0x00000102  # FakeTopicOne writer
$w2 = 0x00000103  # FakeTopicTwo writer
$rd = 0x00000107  # FakeTopicOne reader
$w4 = 0x00000104  # HelloWorldTopic writer (registered)
$sedpPub = 0x000003c2
$sedpSub = 0x000004c2
$spdp = 0x000100c2
$user1 = [byte[]]@(0x00, 0x01, 0x00, 0x00, 0x2a, 0x00, 0x00, 0x00)  # CDR_LE + a word
$user2 = [byte[]]@(0x00, 0x01, 0x00, 0x00, 0x2b, 0x00, 0x00, 0x00)

# Datagram 1: coalesced SEDP announcements (three DATA) + INFO_TS + HEARTBEAT.
$d1 = Concat @(
    (RtpsHeader $prefix),
    (InfoTs),
    (DataSub $sedpPub 1 (EndpointPayload $prefix $w1 'FakeTopicOne' 'fake::FakeTopicOne')),
    (DataSub $sedpPub 2 (EndpointPayload $prefix $w2 'FakeTopicTwo' 'fake::FakeTopicTwo')),
    (DataSub $sedpSub 1 (EndpointPayload $prefix $rd 'FakeTopicOne' 'fake::FakeTopicOne')),
    (DataSub $sedpPub 4 (EndpointPayload $prefix $w4 'HelloWorldTopic' 'hello_world::HelloWorldTopic')),
    (Heartbeat $sedpPub 1 4 1))

# Datagram 2: an SPDP participant announcement + HEARTBEAT.
$d2 = Concat @(
    (RtpsHeader $prefix),
    (InfoTs),
    (DataSub $spdp 1 (ParticipantPayload $prefix)),
    (Heartbeat $spdp 1 1 1))

# Datagram 3: user data for FakeTopicOne + INFO_DST + HEARTBEAT.
$d3 = Concat @(
    (RtpsHeader $prefix),
    (InfoTs),
    (InfoDst $prefix),
    (DataSub $w1 1 $user1),
    (Heartbeat $w1 1 1 1))

# Datagram 4: user data for FakeTopicTwo + INFO_SRC + ACKNACK + HEARTBEAT.
$d4 = Concat @(
    (RtpsHeader $prefix),
    (InfoSrc $prefix),
    (DataSub $w2 1 $user2),
    (AckNack $w2 1),
    (Heartbeat $w2 1 1 1))

# Datagram 5: a mixed bag exercising PAD, INFO_DST, DATA_FRAG, GAP and an
# unknown submessage id, plus more FakeTopicOne data.
$d5 = Concat @(
    (RtpsHeader $prefix),
    (Pad),
    (InfoDst $prefix),
    (DataFrag $w1 2),
    (DataSub $w1 2 $user1),
    (Gap $sedpPub 4),
    (Sub 0x99 0x01 ([byte[]]@(1, 2, 3, 4, 5, 6))),
    (Heartbeat $w1 2 2 1))

# Datagram 6: a decodable HelloWorldTopic sample (a registered/known topic).
$d6 = Concat @(
    (RtpsHeader $prefix),
    (InfoTs),
    (DataSub $w4 1 (CdrHelloWorldTopic 'Hello, world!' 0.0 0.0 0.0 1.0)),
    (Heartbeat $w4 1 1 1))

$datagrams = @($d1, $d2, $d3, $d4, $d5, $d6)

$sock = [System.Net.Sockets.Socket]::new(
    [System.Net.Sockets.AddressFamily]::InterNetworkV6,
    [System.Net.Sockets.SocketType]::Dgram,
    [System.Net.Sockets.ProtocolType]::Udp)
try {
    $sock.Bind([System.Net.IPEndPoint]::new([System.Net.IPAddress]::Parse($addr.IPAddress), 0))
    $dest = [System.Net.IPEndPoint]::new([System.Net.IPAddress]::Parse("$Group%$ifindex"), $Port)

    Write-Host "Sending $($datagrams.Count) crafted datagrams to [$Group%$ifindex]:$Port ($Repeat time(s))."
    Write-Host "Start dds-echo, press Start, and expand the 'Unknown' topics group."
    for ($i = 1; $i -le $Repeat; $i++) {
        foreach ($d in $datagrams) {
            [void]$sock.SendTo($d, $dest)
            Start-Sleep -Milliseconds $DelayMs
        }
    }
    Write-Host "Expected: HelloWorldTopic under Known with the decoded fields in the detail pane;"
    Write-Host "FakeTopicOne/FakeTopicTwo/unidentified under Unknown; each datagram's detail pane shows"
    Write-Host "its submessage envelopes with any decoded announcement beneath them."
}
finally {
    $sock.Close()
}
