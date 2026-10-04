//! Small real-MCP-process regressions for shared lifecycle and delivery state.
use rmcp::{model::CallToolRequestParam, transport::TokioChildProcess, ServiceExt};
use serde_json::{json, Value};
use std::{path::Path, time::Duration};
use uuid::Uuid;
use warp_agent_bus::{
    readiness::Activity,
    transport::{self, Request, RunningBroker, CAPABILITY, ENDPOINT, TERMINAL},
    Operation,
};

async fn call(server: &RunningBroker, request: &Request) -> Value {
    let endpoint = server.broker.endpoint.clone();
    let request = request.clone();
    tokio::task::spawn_blocking(move || transport::call(&endpoint, &request).unwrap())
        .await
        .unwrap()
}

async fn status(server: &RunningBroker, observer: &Request, terminal: &str) -> Value {
    let mut request = observer.clone();
    request.operation = Operation::AgentList;
    call(server, &request)
        .await
        .as_array()
        .unwrap()
        .iter()
        .find(|peer| peer["terminal"] == terminal)
        .unwrap()
        .clone()
}

fn programs() -> Vec<&'static str> {
    let mut programs: Vec<_> = include_str!("../../../app/src/terminal/cli_agent.rs")
        .lines()
        .filter_map(|line| line.trim().strip_prefix("CLIAgent::"))
        .filter(|line| line.contains("=> &[\""))
        .filter_map(|line| line.split('"').nth(1))
        .collect();
    programs.push("custom");
    programs
}

#[tokio::test]
async fn every_program_keeps_queries_and_drafts_ready_and_recovers_after_work() {
    let server = RunningBroker::start(Path::new(":memory:")).unwrap();
    let capability = server.broker.prepare("observer").unwrap();
    server
        .broker
        .activate("observer", "codex", "/project", true)
        .unwrap();
    let mut observer = Request {
        protocol_major: warp_agent_bus::transport::PROTOCOL_MAJOR,
        terminal: "observer".into(),
        capability,
        run: None,
        defer_initial_ready: false,
        native_activity: None,
        directory: None,
        operation: Operation::AgentRegister {
            name: "observer".into(),
        },
    };
    observer.run = call(&server, &observer).await["run"]
        .as_str()
        .map(str::to_owned);
    let programs = programs();
    assert!(programs.len() >= 19);
    for (index, program) in programs.into_iter().enumerate() {
        let terminal = format!("worker-{index}");
        let capability = server.broker.prepare(&terminal).unwrap();
        server
            .broker
            .activate(&terminal, program, "/project", true)
            .unwrap();
        let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_warpai-agent"));
        command
            .arg("mcp")
            .env(ENDPOINT, &server.broker.endpoint)
            .env(CAPABILITY, &capability)
            .env(TERMINAL, &terminal);
        let client = ().serve(TokioChildProcess::new(command).unwrap()).await.unwrap();
        client.list_tools(None).await.unwrap();
        let mut worker = Request {
            protocol_major: warp_agent_bus::transport::PROTOCOL_MAJOR,
            terminal: terminal.clone(),
            capability,
            run: None,
            defer_initial_ready: false,
            native_activity: None,
            directory: None,
            operation: Operation::AgentRegister {
                name: String::new(),
            },
        };
        worker.run = call(&server, &worker).await["run"]
            .as_str()
            .map(str::to_owned);
        let peer = status(&server, &observer, &terminal).await;
        assert_eq!(peer["ready"], true, "fresh {program}");

        for tool in ["warp_agent_list", "warp_agent_inbox"] {
            client
                .call_tool(CallToolRequestParam {
                    name: tool.into(),
                    arguments: None,
                })
                .await
                .unwrap();
            assert_eq!(
                status(&server, &observer, &terminal).await["ready"],
                true,
                "query {tool} on {program}"
            );
        }
        server.broker.input_bytes(&terminal, "几个字".as_bytes());
        let drafted = status(&server, &observer, &terminal).await;
        assert_eq!(drafted["ready"], true);
        assert_eq!(drafted["has_draft"], true);
        assert_eq!(drafted["can_auto_submit"], false);
        server.broker.input_bytes(&terminal, b"\x7f\x7f\x7f");
        let cleared = status(&server, &observer, &terminal).await;
        assert_eq!(cleared["ready"], true);
        assert_eq!(cleared["draft_state"], "empty");
        assert_eq!(
            cleared["readiness_source"], "startup",
            "editing must not re-arm readiness"
        );

        for turn in 0..2 {
            observer.operation = serde_json::from_value(json!({"op":"agent_send","to":peer["id"],"body":"Report the weekday","request_id":Uuid::new_v4().to_string()})).unwrap();
            let message = call(&server, &observer).await;
            tokio::time::sleep(Duration::from_millis(800)).await;
            let wake = server
                .broker
                .wakeups()
                .into_iter()
                .find(|wake| wake.terminal == terminal)
                .unwrap();
            assert!(server.broker.claim_wake(&wake));
            assert_eq!(status(&server, &observer, &terminal).await["ready"], true);
            server.broker.finish_wake(&wake, true);
            assert_eq!(
                status(&server, &observer, &terminal).await["ready"],
                false,
                "dispatch turn {turn} on {program}"
            );
            client
                .call_tool(CallToolRequestParam {
                    name: "warp_agent_inbox".into(),
                    arguments: None,
                })
                .await
                .unwrap();
            client
                .call_tool(CallToolRequestParam {
                    name: "warp_agent_ack".into(),
                    arguments: Some(
                        json!({"message_id":message["id"]})
                            .as_object()
                            .unwrap()
                            .clone(),
                    ),
                })
                .await
                .unwrap();
            client
                .call_tool(CallToolRequestParam {
                    name: "warp_agent_ready".into(),
                    arguments: None,
                })
                .await
                .unwrap();
            assert_eq!(
                status(&server, &observer, &terminal).await["ready"],
                true,
                "finish turn {turn} on {program}"
            );
        }
        for activity in [
            Activity::Working,
            Activity::WaitingApproval,
            Activity::WaitingInput,
            Activity::Error,
            Activity::Starting,
        ] {
            worker.native_activity = Some(activity);
            worker.operation = Operation::AgentList;
            call(&server, &worker).await;
            client
                .call_tool(CallToolRequestParam {
                    name: "warp_agent_ready".into(),
                    arguments: None,
                })
                .await
                .unwrap();
            let peer = status(&server, &observer, &terminal).await;
            assert_eq!(peer["ready"], false, "native {activity:?} on {program}");
            assert_eq!(peer["activity"], json!(activity));
            client.list_tools(None).await.unwrap();
            assert_eq!(
                status(&server, &observer, &terminal).await["ready"],
                false,
                "rediscovery {program}"
            );
        }
        worker.native_activity = Some(Activity::Idle);
        call(&server, &worker).await;
        server.broker.input_bytes(&terminal, b"own task\r");
        assert_eq!(status(&server, &observer, &terminal).await["ready"], false);
        call(&server, &worker).await;
        assert_eq!(
            status(&server, &observer, &terminal).await["ready"],
            true,
            "local completion {program}"
        );
        // Completion cannot discard input typed while the preceding turn was busy.
        server.broker.input_bytes(&terminal, b"unsent");
        call(&server, &worker).await;
        assert_eq!(
            status(&server, &observer, &terminal).await["has_draft"],
            true
        );
        server.broker.input_guard(&terminal, false, true);
        assert_eq!(
            status(&server, &observer, &terminal).await["can_auto_submit"],
            false
        );
        server.broker.input_guard(&terminal, true, false);
        assert_eq!(status(&server, &observer, &terminal).await["ready"], true);
        server.broker.input_guard(&terminal, false, false);
        server.broker.input_bytes(&terminal, b"\x03");
        call(&server, &worker).await;
        let paused = status(&server, &observer, &terminal).await;
        assert_eq!(paused["paused"], true);
        assert_eq!(paused["can_auto_submit"], false);
        server.broker.input_bytes(&terminal, b"resume\r");
        call(&server, &worker).await;
        assert_eq!(status(&server, &observer, &terminal).await["paused"], false);
        server.broker.expire_epoch(&terminal);
        assert_eq!(
            status(&server, &observer, &terminal).await["can_auto_submit"],
            false
        );
        client.cancel().await.unwrap();
        server.broker.end(&terminal);
    }
}

#[tokio::test]
async fn assigned_task_completion_restores_readiness_without_another_turn() {
    let server = RunningBroker::start(Path::new(":memory:")).unwrap();
    let mut peers = Vec::new();
    for terminal in ["issuer", "worker", "observer"] {
        let capability = server.broker.prepare(terminal).unwrap();
        server
            .broker
            .activate(terminal, "qodercn", "/project", true)
            .unwrap();
        let mut request = Request {
            protocol_major: warp_agent_bus::transport::PROTOCOL_MAJOR,
            terminal: terminal.into(),
            capability,
            run: None,
            defer_initial_ready: false,
            native_activity: None,
            directory: None,
            operation: Operation::AgentRegister {
                name: terminal.into(),
            },
        };
        request.run = call(&server, &request).await["run"]
            .as_str()
            .map(str::to_owned);
        peers.push(request);
    }
    for _ in 0..2 {
        peers[0].operation = serde_json::from_value(json!({"op":"task_assign","to":"worker","description":"Inspect weekday","acceptance":"Return weekday","request_id":Uuid::new_v4().to_string()})).unwrap();
        let task = call(&server, &peers[0]).await;
        peers[1].operation = serde_json::from_value(json!({"op":"task_start","task_id":task["id"],"revision":1,"request_id":Uuid::new_v4().to_string()})).unwrap();
        let start_request = peers[1].clone();
        call(&server, &peers[1]).await;
        assert_eq!(status(&server, &peers[0], "worker").await["ready"], false);
        peers[1].operation = Operation::AgentReady;
        assert_eq!(
            call(&server, &peers[1]).await["ready"],
            false,
            "A model cannot announce idle during its active delegated execution"
        );
        assert_eq!(status(&server, &peers[0], "worker").await["ready"], false);
        server.broker.readiness("worker", true);
        tokio::time::sleep(Duration::from_millis(850)).await;
        let hidden = status(&server, &peers[2], "worker").await;
        assert!(hidden["tasks"].as_array().unwrap().is_empty());
        assert_eq!(hidden["can_auto_submit"], true);
        assert_eq!(
            hidden["can_start_task"], false,
            "Private task content must stay hidden without advertising another execution grant"
        );
        peers[1].operation = serde_json::from_value(json!({"op":"task_submit","task_id":task["id"],"revision":1,"result":"Friday","evidence":"Calendar checked","request_id":Uuid::new_v4().to_string()})).unwrap();
        let result = call(&server, &peers[1]).await;
        assert!(result["instruction"]
            .as_str()
            .unwrap()
            .contains("warp_agent_ready"));
        assert_eq!(status(&server, &peers[0], "worker").await["ready"], true);
        call(&server, &start_request).await;
        assert_eq!(
            status(&server, &peers[0], "worker").await["ready"],
            true,
            "Replaying an old start must not revoke readiness after submission"
        );
        let submission_request = peers[1].clone();
        server.broker.input_bytes("worker", b"New user work\r");
        assert_eq!(status(&server, &peers[0], "worker").await["ready"], false);
        call(&server, &submission_request).await;
        assert_eq!(
            status(&server, &peers[0], "worker").await["ready"],
            false,
            "An identical submission replay must not make a later user turn idle"
        );
        peers[0].operation = serde_json::from_value(json!({"op":"task_review","task_id":task["id"],"revision":1,"accepted":false,"feedback":"Repeat the check","request_id":Uuid::new_v4().to_string()})).unwrap();
        call(&server, &peers[0]).await;
        peers[1].operation = serde_json::from_value(json!({"op":"task_start","task_id":task["id"],"revision":2,"request_id":Uuid::new_v4().to_string()})).unwrap();
        call(&server, &peers[1]).await;
        call(&server, &submission_request).await;
        assert_eq!(
            status(&server, &peers[0], "worker").await["ready"],
            false,
            "Replaying an old submission must not announce idle during a newer attempt"
        );
        peers[1].operation = serde_json::from_value(json!({"op":"task_submit","task_id":task["id"],"revision":2,"result":"Friday","evidence":"Calendar checked again","request_id":Uuid::new_v4().to_string()})).unwrap();
        call(&server, &peers[1]).await;
        peers[0].operation = serde_json::from_value(json!({"op":"task_review","task_id":task["id"],"revision":2,"accepted":true,"feedback":"Verified","request_id":Uuid::new_v4().to_string()})).unwrap();
        call(&server, &peers[0]).await;
    }
    peers[0].operation = serde_json::from_value(json!({"op":"task_create_pool","description":"Inspect weekday","acceptance":"Return weekday","eligible":["worker"],"request_id":Uuid::new_v4().to_string()})).unwrap();
    let task = call(&server, &peers[0]).await;
    peers[1].operation = serde_json::from_value(
        json!({"op":"task_claim","task_id":task["id"],"request_id":Uuid::new_v4().to_string()}),
    )
    .unwrap();
    call(&server, &peers[1]).await;
    assert_eq!(status(&server, &peers[0], "worker").await["ready"], true);
}

/// Opt-in authenticated vendor check: one PTY/frontend at a time, with isolated broker state.
#[cfg(target_os = "macos")]
#[test]
#[ignore = "starts installed vendor clients and requests two small model turns"]
fn native_clients_complete_two_turns() {
    use std::{
        io::Write,
        process::{Command, Stdio},
        time::Instant,
    };
    let executable = std::env::var("WARP_READINESS_NATIVE")
        .expect("Set WARP_READINESS_NATIVE to an absolute installed CLI path");
    let program = Path::new(&executable)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap();
    assert!(matches!(program, "codex" | "qodercn"));
    let temporary = tempfile::tempdir().unwrap();
    let repository = temporary.path().join("fixture repository");
    std::fs::create_dir(&repository).unwrap();
    assert!(Command::new("git")
        .args(["init", "--quiet"])
        .arg(&repository)
        .status()
        .unwrap()
        .success());
    let project = warp_agent_bus::project_root(&repository).unwrap();
    let bridge = std::env::var_os("WARP_ACCEPTANCE_BRIDGE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_warpai-agent").into());
    assert!(Path::new(&executable).is_absolute() && bridge.is_absolute());
    assert!(bridge.is_file());
    let version = Command::new(&executable).arg("--version").output().unwrap();
    assert!(version.status.success(), "Native version probe failed");
    println!(
        "Native acceptance: {program}; OS: {}; CLI version: {}",
        std::env::consts::OS,
        String::from_utf8_lossy(&version.stdout)
            .lines()
            .next()
            .unwrap_or("unknown")
    );
    let server = RunningBroker::start(Path::new(":memory:")).unwrap();
    let mut observer = Request {
        protocol_major: warp_agent_bus::transport::PROTOCOL_MAJOR,
        terminal: "observer".into(),
        capability: server.broker.prepare("observer").unwrap(),
        run: None,
        defer_initial_ready: false,
        native_activity: None,
        directory: None,
        operation: Operation::AgentRegister {
            name: "observer".into(),
        },
    };
    server
        .broker
        .activate("observer", "codex", &project, true)
        .unwrap();
    observer.run = transport::call(&server.broker.endpoint, &observer).unwrap()["run"]
        .as_str()
        .map(str::to_owned);
    observer.operation = Operation::AgentList;
    let capability = server.broker.prepare("native").unwrap();
    server
        .broker
        .activate("native", program, &project, true)
        .unwrap();
    let codex_options = serde_json::to_string(&warp_agent_bus::session::codex_mcp_prefix(
        &bridge,
        &["mcp".into()],
        &[ENDPOINT, CAPABILITY, TERMINAL],
    ))
    .unwrap();
    let driver = r#"
import fcntl,json,os,pty,re,select,signal,struct,subprocess,sys,termios,time
markers=set()
def record():
 with open(sys.argv[3],'w') as output: json.dump(sorted(markers),output)
master,slave=pty.openpty()
os.set_blocking(master,False)
fcntl.ioctl(slave,termios.TIOCSWINSZ,struct.pack('HHHH',40,160,0,0))
def attach_terminal():
 os.setsid()
 fcntl.ioctl(slave,termios.TIOCSCTTY,0)
args=[sys.argv[1]]
if os.path.basename(sys.argv[1])=='codex':
 args+=json.loads(os.environ['WARP_READINESS_CODEX_OPTIONS'])
child=subprocess.Popen(args,stdin=slave,stdout=slave,stderr=slave,cwd=sys.argv[2],preexec_fn=attach_terminal)
os.close(slave)
recent=b''
try:
 while child.poll() is None:
  readable,_,_=select.select([master,sys.stdin],[],[],.1)
  if sys.stdin in readable:
   text=sys.stdin.readline()
   if not text: break
   text=text.rstrip('\n').encode()
   if os.path.basename(sys.argv[1])=='codex':
    os.write(master,b'\x1b[200~'+text+b'\x1b[201~'); time.sleep(.05); os.write(master,b'\r')
   else: os.write(master,text+b'\r')
  if master in readable:
   try: data=os.read(master,65536)
   except OSError: break
   markers.add('terminal_output')
   recent=(recent+data)[-65536:]
   lower=re.sub(rb'\x1b\[[0-?]*[ -/]*[@-~]',b'',recent).lower()
   for indicator in [b'trust',b'login required',b'not logged in',b'not authenticated',b'please log in',b'sign in',b'not permitted',b'permission denied',b'handshake not finished',b'error',b'failed',b'network',b'missing',b'connection',b'native session connection timed out',b'native session connection closed',b'connection refused',b'connection reset',b'terminal',b'initializ',b'device',b'closed before',b'cursor position']:
    if indicator in lower: markers.add(indicator.decode())
   record()
   for query,reply in [(b'\x1b[6n',b'\x1b[1;1R'),(b'\x1b[c',b'\x1b[?1;2c'),(b'\x1b[>q',b'\x1bP>|Warpai(1.0)\x1b\\'),(b'\x1b[?u',b'\x1b[?0u'),(b'\x1b]10;?',b'\x1b]10;rgb:ffff/ffff/ffff\x1b\\'),(b'\x1b]11;?',b'\x1b]11;rgb:0000/0000/0000\x1b\\')]:
    if query in data: os.write(master,reply)
finally:
 markers.add('native_exit_'+str(child.poll())); record()
 # The new session belongs only to this probe, including its native grandchildren.
 for sig in [signal.SIGCONT,signal.SIGTERM]:
  try: os.killpg(child.pid,sig)
  except ProcessLookupError: pass
 try: child.wait(timeout=3)
 except subprocess.TimeoutExpired:
  try: os.killpg(child.pid,signal.SIGKILL)
  except ProcessLookupError: pass
  child.wait(timeout=3)
 os.close(master)
"#;
    let diagnostic = temporary.path().join("diagnostic.json");
    let mut child = Command::new("python3")
        .args(["-u", "-c", driver])
        .arg(&executable)
        .arg(&project)
        .arg(&diagnostic)
        .env(ENDPOINT, &server.broker.endpoint)
        .env(CAPABILITY, capability)
        .env(TERMINAL, "native")
        .env("WARP_AGENT_BIN", &bridge)
        .env("WARP_READINESS_CODEX_OPTIONS", &codex_options)
        .env_remove("WARP_AGENT_LAUNCH_PATH")
        .env("TERM", "xterm-256color")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let status = || {
            transport::call(&server.broker.endpoint, &observer)
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .find(|peer| peer["terminal"] == "native")
                .cloned()
        };
        let wait_ready = |child: &mut std::process::Child| {
            let deadline = Instant::now() + Duration::from_secs(60);
            loop {
                if let Some(peer) = status() {
                    if peer["ready"] == true {
                        return peer;
                    }
                }
                assert!(
                    child.try_wait().unwrap().is_none() && Instant::now() < deadline,
                    "{program} did not return idle: {:?}; indicators: {}",
                    status(),
                    std::fs::read_to_string(&diagnostic).unwrap_or_default()
                );
                std::thread::sleep(Duration::from_millis(100));
            }
        };
        wait_ready(&mut child);
        for turn in 0..2 {
            std::thread::sleep(Duration::from_millis(1500));
            // Represents the app's real input submission; no automatic approval is made.
            server.broker.input_bytes("native", b"weekday question\r");
            writeln!(child.stdin.as_mut().unwrap(), "Reply only with the weekday for 2026-10-02. Before finishing, call warp_agent_ready as your final tool action. Do not change any files.").unwrap();
            assert_eq!(status().unwrap()["ready"], false);
            let peer = wait_ready(&mut child);
            println!(
                "{program}: turn {} completed, ready={}, source={}",
                turn + 1,
                peer["ready"],
                peer["readiness_source"]
            );
        }
    }));
    drop(child.stdin.take());
    let cleanup_deadline = Instant::now() + Duration::from_secs(10);
    while child.try_wait().unwrap().is_none() && Instant::now() < cleanup_deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    if child.try_wait().unwrap().is_none() {
        child.kill().unwrap();
    }
    child.wait().unwrap();
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

/// A vendor executable name must never turn the owned MCP bridge into that vendor.
#[cfg(unix)]
#[test]
fn bridge_cannot_impersonate_codex() {
    use std::os::unix::process::CommandExt;
    let launch =
        json!({"codex": {"executable": "/usr/bin/true", "program": "codex", "options": {}}});
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_warpai-agent"))
        .arg0("codex")
        .arg("--help")
        .env("WARP_AGENT_LAUNCHES", launch.to_string())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Usage: warpai-agent"));
}
