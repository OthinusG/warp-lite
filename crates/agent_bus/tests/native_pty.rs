//! Actual native terminal process, root/Unicode IO, resize and observed exit.
use std::{
    io::{Read, Write},
    time::{Duration, Instant},
};
use warp_agent_bus::native_pty::{NativePty, Size};

#[test]
#[ignore = "child invoked only by the owned PTY parent"]
fn native_terminal_child() {
    #[cfg(unix)]
    assert_eq!(unsafe { libc::isatty(0) }, 1);
    #[cfg(windows)]
    unsafe {
        use windows::Win32::System::Console::{GetConsoleMode, GetStdHandle, STD_INPUT_HANDLE};
        let mut mode = 0;
        GetConsoleMode(GetStdHandle(STD_INPUT_HANDLE).unwrap(), &mut mode).unwrap();
    }
    println!("PTY_CHILD_READY");
    let mut root = String::new();
    std::io::stdin().read_line(&mut root).unwrap();
    assert_eq!(
        std::env::current_dir().unwrap().canonicalize().unwrap(),
        std::path::Path::new(root.trim_end())
            .canonicalize()
            .unwrap()
    );
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    assert_eq!(input.trim_end(), "Unicode 空格;$(never-execute)");
    println!("PTY_ROOT_AND_UNICODE_VERIFIED");
}

#[test]
fn native_terminal_owns_real_process_root_io_resize_and_exit() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("Native terminal 多语言 with spaces");
    std::fs::create_dir(&root).unwrap();
    let executable = std::env::current_exe().unwrap();
    let args = [
        "--exact",
        "native_terminal_child",
        "--ignored",
        "--nocapture",
    ]
    .map(str::to_owned);
    assert!(NativePty::spawn(
        &executable,
        &args,
        &root,
        Size {
            columns: 0,
            rows: 24
        }
    )
    .is_err());
    let mut pty = NativePty::spawn(
        &executable,
        &args,
        &root,
        Size {
            columns: 80,
            rows: 24,
        },
    )
    .unwrap();
    let mut reader = pty.reader.try_clone().unwrap();
    let (sender, receiver) = std::sync::mpsc::sync_channel(16);
    let thread = std::thread::spawn(move || {
        let mut buffer = [0; 4096];
        while let Ok(count) = reader.read(&mut buffer) {
            if count == 0 || sender.send(buffer[..count].to_vec()).is_err() {
                break;
            }
        }
    });
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut output = Vec::new();
    loop {
        let bytes = receiver
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
        output.extend(bytes);
        assert!(output.len() < 65536);
        if String::from_utf8_lossy(&output).contains("PTY_CHILD_READY") {
            break;
        }
    }
    pty.resize(Size {
        columns: 120,
        rows: 40,
    })
    .unwrap();
    pty.writer
        .write_all(
            format!(
                "{}\rUnicode 空格;$(never-execute)\r",
                root.to_str().unwrap()
            )
            .as_bytes(),
        )
        .unwrap();
    pty.writer.flush().unwrap();
    let code = loop {
        if let Some(code) = pty.exit_code().unwrap() {
            break code;
        }
        assert!(
            Instant::now() < deadline,
            "Owned native terminal did not exit"
        );
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(code, 0);
    assert!(!pty.stop().unwrap());
    drop(pty);
    drop(receiver);
    thread.join().unwrap();
}
