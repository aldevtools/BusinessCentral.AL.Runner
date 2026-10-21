//! The wire of the Debug Adapter Protocol (ADR-048): `Content-Length` framing of UTF-8 JSON, as v2's `--dap` speaks it.
//! Requests come in on one stream; responses and events go out on another under one lock, numbered by one counter.

use serde_json::{Map, Value, json};
use std::io::{BufRead, Write};
use std::sync::Mutex;

/// Reads one message. `Ok(None)`: the stream ended before a message began.
pub fn read_message(r: &mut impl BufRead) -> Result<Option<Value>, String> {
    let mut length: Option<usize> = None;
    let mut first = true;
    loop {
        let mut line = String::new();
        let n = r.read_line(&mut line).map_err(|e| format!("read: {e}"))?;
        if n == 0 {
            return if first && length.is_none() {
                Ok(None)
            } else {
                Err("the stream ended inside a message header".into())
            };
        }
        first = false;
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            break;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(format!("not a header line: `{line}`"));
        };
        if name.trim().eq_ignore_ascii_case("Content-Length") {
            length = Some(
                value
                    .trim()
                    .parse()
                    .map_err(|_| format!("not a length: `{}`", value.trim()))?,
            );
        }
    }
    let Some(length) = length else {
        return Err("a message header without Content-Length".into());
    };
    let mut body = vec![0u8; length];
    r.read_exact(&mut body)
        .map_err(|e| format!("the stream ended inside a message body of {length} bytes: {e}"))?;
    serde_json::from_slice(&body)
        .map(Some)
        .map_err(|e| format!("a message body that is not JSON: {e}"))
}

/// Writes one message with its header.
pub fn write_message(w: &mut dyn Write, msg: &Value) -> std::io::Result<()> {
    let body = serde_json::to_vec(msg).unwrap_or_default();
    write!(w, "Content-Length: {}\r\n\r\n", body.len())?;
    w.write_all(&body)?;
    w.flush()
}

/// The outgoing stream: responses and events, numbered.
pub struct Out {
    inner: Mutex<(Box<dyn Write + Send>, i64)>,
}

impl Out {
    pub fn new(w: Box<dyn Write + Send>) -> Out {
        Out {
            inner: Mutex::new((w, 0)),
        }
    }

    fn send(&self, mut msg: Map<String, Value>) {
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        g.1 += 1;
        msg.insert("seq".into(), json!(g.1));
        // A client that went away ends the session through the reader; a failed write changes nothing here.
        let _ = write_message(&mut *g.0, &Value::Object(msg));
    }

    /// A response to request `seq`. `body` and `message` are left out when `None`.
    pub fn response(&self, seq: i64, command: &str, success: bool, body: Option<Value>, message: Option<&str>) {
        let mut m = Map::new();
        m.insert("type".into(), json!("response"));
        m.insert("request_seq".into(), json!(seq));
        m.insert("success".into(), json!(success));
        m.insert("command".into(), json!(command));
        if let Some(t) = message {
            m.insert("message".into(), json!(t));
        }
        if let Some(b) = body {
            m.insert("body".into(), b);
        }
        self.send(m);
    }

    pub fn ok(&self, seq: i64, command: &str, body: Option<Value>) {
        self.response(seq, command, true, body, None);
    }

    pub fn fail(&self, seq: i64, command: &str, message: &str) {
        self.response(seq, command, false, None, Some(message));
    }

    pub fn event(&self, event: &str, body: Option<Value>) {
        let mut m = Map::new();
        m.insert("type".into(), json!("event"));
        m.insert("event".into(), json!(event));
        if let Some(b) = body {
            m.insert("body".into(), b);
        }
        self.send(m);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(bytes: &[u8]) -> Result<Option<Value>, String> {
        read_message(&mut std::io::BufReader::new(bytes))
    }

    #[test]
    fn a_framed_message_reads_back() {
        let mut buf = Vec::new();
        write_message(&mut buf, &json!({"command": "initialize", "seq": 1})).unwrap();
        assert!(buf.starts_with(b"Content-Length: "));
        assert_eq!(read(&buf).unwrap().unwrap()["command"], "initialize");
    }

    #[test]
    fn two_messages_in_one_stream_read_one_after_the_other() {
        let mut buf = Vec::new();
        write_message(&mut buf, &json!({"seq": 1})).unwrap();
        write_message(&mut buf, &json!({"seq": 2})).unwrap();
        let mut r = std::io::BufReader::new(&buf[..]);
        assert_eq!(read_message(&mut r).unwrap().unwrap()["seq"], 1);
        assert_eq!(read_message(&mut r).unwrap().unwrap()["seq"], 2);
        assert_eq!(read_message(&mut r).unwrap(), None);
    }

    #[test]
    fn other_headers_and_any_case_are_accepted() {
        let body = br#"{"seq":7}"#;
        let msg = format!(
            "content-length: {}\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\n\r\n",
            body.len()
        );
        let mut buf = msg.into_bytes();
        buf.extend_from_slice(body);
        assert_eq!(read(&buf).unwrap().unwrap()["seq"], 7);
    }

    #[test]
    fn a_short_body_a_missing_length_and_a_bad_length_are_errors() {
        assert!(read(b"Content-Length: 10\r\n\r\n{}").unwrap_err().contains("10 bytes"));
        assert!(
            read(b"X-Other: 1\r\n\r\n{}")
                .unwrap_err()
                .contains("without Content-Length")
        );
        assert!(
            read(b"Content-Length: ten\r\n\r\n")
                .unwrap_err()
                .contains("not a length")
        );
        assert!(
            read(b"Content-Length: 2\r\n")
                .unwrap_err()
                .contains("inside a message header")
        );
        assert_eq!(read(b"").unwrap(), None);
    }

    #[test]
    fn responses_and_events_are_numbered_and_leave_out_what_is_not_given() {
        let out = Out::new(Box::new(Vec::new()));
        out.ok(3, "threads", None);
        out.event("initialized", None);
        let g = out.inner.lock().unwrap();
        assert_eq!(g.1, 2);
    }
}
