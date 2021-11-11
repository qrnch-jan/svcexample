use std::env;
use std::fs;
use std::{ffi::OsString, time::Duration};

use windows_service::{
  define_windows_service,
  service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState,
    ServiceStatus, ServiceType
  },
  service_control_handler::{self, ServiceControlHandlerResult},
  service_dispatcher
};

use figment::Figment;
use figment_winreg::RegistryProvider;

use serde::Deserialize;

use log::{debug, error, info};

use winreg::{enums::*, RegKey};

use qargparser as arg;

use crate::err::Error;

const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;
const SERVICE_STARTPENDING_TIME: Duration = Duration::from_secs(30);
const SERVICE_STOPPENDING_TIME: Duration = Duration::from_secs(30);

#[derive(Default)]
pub struct Context {
  svcname: String
}

#[derive(Deserialize, Debug)]
pub struct Config {
  #[serde(rename = "LogLevel")]
  loglevel: Option<String>
}

/// Kick off service dispatch loop.
pub fn run() -> Result<(), Error> {
  let ctx = parse()?;

  service_dispatcher::start(ctx.svcname, ffi_service_main)?;

  Ok(())
}

// Generate the windows service boilerplate.  The boilerplate contains the
// low-level service entry function (ffi_service_main) that parses incoming
// service arguments into Vec<OsString> and passes them to user defined service
// entry (my_service_main).
define_windows_service!(ffi_service_main, my_service_main);

fn my_service_main(_arguments: Vec<OsString>) {
  let _ = inner_main();
}

/// Service entry function which is called on background thread by the system
/// with service parameters.  There is no stdout or stderr at this point so
/// make sure to configure the log.
fn inner_main() -> Result<(), Error> {
  let ctx = match parse() {
    Ok(ctx) => ctx,
    Err(_) => panic!("Unable to parse command line")
  };

  let pth = format!(
    r"SYSTEM\CurrentControlSet\Services\{}\Parameters",
    ctx.svcname
  );

  let fig =
    Figment::new().merge(RegistryProvider::new(HKEY_LOCAL_MACHINE, pth));
  let config: Config = fig.extract()?;

  // Set up logging level from registry parameter.
  // Defaults to error level
  let lf = if let Some(ll) = config.loglevel {
    match ll.as_ref() {
      "off" => log::LevelFilter::Off,
      "error" => log::LevelFilter::Error,
      "warn" => log::LevelFilter::Warn,
      "info" => log::LevelFilter::Info,
      "debug" => log::LevelFilter::Debug,
      "trace" => log::LevelFilter::Trace,
      _ => log::LevelFilter::Error
    }
  } else {
    log::LevelFilter::Error
  };

  // Initailize log
  eventlog::init(&ctx.svcname, log::Level::Trace).unwrap();
  log::set_max_level(lf);

  debug!("Setting up service");

  // Define system service event handler that will be receiving service events.
  // Don't currently handle stop events.
  let event_handler = move |control_event| -> ServiceControlHandlerResult {
    match control_event {
      ServiceControl::Interrogate => {
        debug!("svc signal recieved: interrogate");
        // Notifies a service to report its current status information to the
        // service control manager. Always return NoError even if not
        // implemented.
        ServiceControlHandlerResult::NoError
      }
      ServiceControl::Stop => {
        debug!("svc signal recieved: stop");
        ServiceControlHandlerResult::NoError
      }
      ServiceControl::Continue => {
        debug!("svc signal recieved: continue");
        ServiceControlHandlerResult::NotImplemented
      }
      ServiceControl::Pause => {
        debug!("svc signal recieved: pause");
        ServiceControlHandlerResult::NotImplemented
      }
      _ => {
        debug!("svc signal recieved: other");
        ServiceControlHandlerResult::NotImplemented
      }
    }
  };

  // Switch working directory if set.
  if let Some(wd) = get_service_param(&ctx.svcname, "WorkDir") {
    env::set_current_dir(wd)?;
  }

  // Register system service event handler.  (The returned status handle
  // should be used to report service status changes to the system).  And
  // report that we're in the process of starting up.
  let status_handle =
    service_control_handler::register(&ctx.svcname, event_handler)?;

  status_handle.set_service_status(ServiceStatus {
    service_type: SERVICE_TYPE,
    current_state: ServiceState::StartPending,
    controls_accepted: ServiceControlAccept::empty(),
    exit_code: ServiceExitCode::Win32(0),
    checkpoint: 0,
    wait_hint: SERVICE_STARTPENDING_TIME,
    process_id: None
  })?;

  status_handle.set_service_status(ServiceStatus {
    service_type: SERVICE_TYPE,
    current_state: ServiceState::Running,
    controls_accepted: ServiceControlAccept::STOP,
    exit_code: ServiceExitCode::Win32(0),
    checkpoint: 0,
    wait_hint: Duration::default(),
    process_id: None
  })?;

  // Wait for a debugger to attach, and then break
  #[cfg(feature = "dbgtools-win")]
  dbgtools_win::debugger::wait_for_then_break();

  let caplog = r"C:\Temp\service.log";
  //let program = "powershell.exe";
  let program = r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe";
  let args = &[
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    r"C:\Temp\service.ps1"
  ];
  match fs::OpenOptions::new()
    .create(true)
    .write(true)
    .append(true)
    .open(caplog)
  {
    Ok(f) => {
      info!("Running command!");
      match duct::cmd(program, args)
        .stderr_to_stdout()
        .unchecked()
        .stdout_file(f)
        .run()
      {
        Ok(_) => info!("Successfully ran handler script."),
        Err(e) => error!("Unable to run handler; {}", e)
      }
    }
    Err(e) => {
      error!("Unable to open file; {}", e);
    }
  }

  status_handle
    .set_service_status(ServiceStatus {
      service_type: SERVICE_TYPE,
      current_state: ServiceState::StopPending,
      controls_accepted: ServiceControlAccept::empty(),
      exit_code: ServiceExitCode::Win32(0),
      checkpoint: 0,
      wait_hint: SERVICE_STOPPENDING_TIME,
      process_id: None
    })
    .unwrap();

  status_handle
    .set_service_status(ServiceStatus {
      service_type: SERVICE_TYPE,
      current_state: ServiceState::Stopped,
      controls_accepted: ServiceControlAccept::empty(),
      exit_code: ServiceExitCode::Win32(0),
      checkpoint: 0,
      wait_hint: Duration::default(),
      process_id: None
    })
    .unwrap();

  debug!("service terminated");

  Ok(())
}

pub fn parse() -> Result<Context, Error> {
  let ctx = Context {
    ..Default::default()
  };

  let mut prsr = arg::Parser::from_env(ctx);

  prsr.add(
    arg::Builder::new()
      .name("subcmd")
      .required(true)
      .help(&["Subcommand.  Will be set to run-service."])
      .nargs(arg::Nargs::Count(1), &["CMD"])
      .build(|_spec, _ctx: &mut Context, _args| {})
  )?;

  prsr.add(
    arg::Builder::new()
      .name("name")
      .required(true)
      .help(&["Service name."])
      .nargs(arg::Nargs::Count(1), &["NAME"])
      .build(|_spec, ctx: &mut Context, args| {
        ctx.svcname = args[0].clone();
      })
  )?;

  prsr.parse()?;

  Ok(prsr.into_ctx())
}

/// Load a service Parameter from the registry.
pub fn get_service_param(service_name: &str, key: &str) -> Option<String> {
  let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
  let services = match hklm.open_subkey("SYSTEM\\CurrentControlSet\\Services")
  {
    Ok(k) => k,
    Err(_) => return None
  };
  let asrv = match services.open_subkey(service_name) {
    Ok(k) => k,
    Err(_) => return None
  };
  let params = match asrv.open_subkey("Parameters") {
    Ok(k) => k,
    Err(_) => return None
  };

  match params.get_value::<String, &str>(key) {
    Ok(v) => Some(v),
    Err(_) => None
  }
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
