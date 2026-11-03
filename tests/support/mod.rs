use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

// Only the remote ClickUp boundary is replaced. Requests go over real HTTP.
pub fn server(script: Vec<(&'static str, u16, &'static str)>) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/api/v2", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let handle = thread::spawn(move || {
        for (expected, status, body) in script {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(4);
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(connection) => break connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            std::time::Instant::now() < deadline,
                            "Missing request: {expected}"
                        );
                        thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                .unwrap();
            let mut bytes = Vec::new();
            let header_end = loop {
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap();
                assert_ne!(n, 0, "Request ended before headers");
                bytes.extend_from_slice(&buf[..n]);
                if let Some(end) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
                    break end + 4;
                }
            };
            let headers = String::from_utf8_lossy(&bytes[..header_end]).to_lowercase();
            let length = headers
                .lines()
                .find_map(|l| l.strip_prefix("content-length: "))
                .map(|n| n.parse::<usize>().unwrap())
                .unwrap_or(0);
            while bytes.len() < header_end + length {
                let mut buf = [0u8; 4096];
                let n = stream.read(&mut buf).unwrap();
                assert_ne!(n, 0, "Request ended before body");
                bytes.extend_from_slice(&buf[..n]);
            }
            let req = String::from_utf8_lossy(&bytes);
            let (expected, expected_body) = expected
                .split_once('\n')
                .map_or((expected, None), |(line, body)| (line, Some(body)));
            assert!(req.starts_with(expected), "Expected {expected}, got {req}");
            if let Some(body) = expected_body {
                assert_eq!(
                    serde_json::from_slice::<serde_json::Value>(&bytes[header_end..]).unwrap(),
                    serde_json::from_str::<serde_json::Value>(body).unwrap()
                );
            }
            assert!(req.to_lowercase().contains("authorization: pk_test"));
            write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        }
    });
    (base, handle)
}
