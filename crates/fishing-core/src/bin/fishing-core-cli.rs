//! Offline process boundary: stdout is exclusively protocol JSON. No native input is linked.
use anyhow::{bail, Context, Result};
use fishing_core::protocol::{self, Payload, Response, Session};
use std::io::{self, BufRead, Read, Write};

const MAX_JSON_BYTES: u64 = 8 * 1024 * 1024;

fn write_response(output: &mut impl Write, response: &Response) -> Result<()> {
    serde_json::to_writer(&mut *output, response)?;
    writeln!(output)?;
    output.flush()?;
    Ok(())
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let output = io::stdout();
    let mut output = output.lock();
    match args.next().as_deref() {
        Some("replay") => {
            let path = args
                .next()
                .context("Usage: fishing-core-cli replay <scenario.json>")?;
            let mut bytes = vec![];
            std::fs::File::open(path)?
                .take(MAX_JSON_BYTES + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() as u64 > MAX_JSON_BYTES {
                bail!("Replay file exceeds the 8 MiB limit");
            }
            let response = protocol::replay(serde_json::from_slice(&bytes)?);
            write_response(&mut output, &response)?;
            if matches!(response.payload, Payload::Error { .. }) {
                std::process::exit(1);
            }
        }
        Some("serve") => {
            let input = io::stdin();
            let mut input = input.lock();
            let mut session = Session::default();
            loop {
                // Bound each line before parsing; oversized streams cannot grow memory indefinitely.
                let mut line = String::new();
                let read = (&mut input).take(MAX_JSON_BYTES + 1).read_line(&mut line)?;
                if read == 0 {
                    break;
                }
                if read as u64 > MAX_JSON_BYTES {
                    write_response(
                        &mut output,
                        &Response::error("Request exceeds the 8 MiB limit"),
                    )?;
                    bail!("Request limit exceeded");
                }
                write_response(&mut output, &session.handle_json(&line))?;
            }
        }
        _ => bail!("Usage: fishing-core-cli replay <scenario.json> | serve"),
    }
    Ok(())
}
