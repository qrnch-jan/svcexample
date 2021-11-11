use std::thread;
use std::time::Duration;

use qargparser as arg;

use windows_service::{
  service::{ServiceAccess, ServiceState},
  service_manager::{ServiceManager, ServiceManagerAccess}
};

use crate::err::Error;


#[derive(Default)]
pub struct Context {
  do_help: bool,
  svcname: String
}

pub fn parse(args: Vec<String>) -> Result<Option<Context>, Error> {
  let ctx = Context {
    ..Default::default()
  };

  let mut prsr = arg::Parser::from_args("uninstall-service", args, ctx);

  prsr.add(
    arg::Builder::new()
      .sopt('h')
      .lopt("help")
      .exit(true)
      .help(&["Show this help and exit."])
      .build(|_spec, ctx: &mut Context, _args| {
        ctx.do_help = true;
      })
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

  if prsr.get_ctx().do_help == true {
    prsr.usage(&mut std::io::stdout());
    return Ok(None);
  }

  Ok(Some(prsr.into_ctx()))
}


pub fn run(args: Vec<String>) -> Result<(), Error> {
  let ctx = match parse(args)? {
    Some(ctx) => ctx,
    None => return Ok(())
  };

  let manager_access = ServiceManagerAccess::CONNECT;
  let service_manager =
    ServiceManager::local_computer(None::<&str>, manager_access)?;
  let service_access =
    ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
  let service = service_manager.open_service(&ctx.svcname, service_access)?;

  // Make sure service is stopped before trying to delete it
  loop {
    let service_status = service.query_status()?;
    if service_status.current_state == ServiceState::Stopped {
      break;
    }
    println!("==> Requesting service '{}' to stop ..", ctx.svcname);
    service.stop()?;
    thread::sleep(Duration::from_secs(2));
  }

  println!("==> Removing service '{}' ..", ctx.svcname);
  service.delete()?;

  println!("==> Deregistering event log source '{}' ..", ctx.svcname);
  eventlog::deregister(&ctx.svcname)?;

  println!("==> Service uninstallation successful");

  Ok(())
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
