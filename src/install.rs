use std::ffi::OsString;

use qargparser as arg;

use windows_service::{
  service::{
    ServiceAccess, ServiceDependency, ServiceErrorControl, ServiceInfo,
    ServiceStartType, ServiceType
  },
  service_manager::{ServiceManager, ServiceManagerAccess}
};

use winreg::{enums::*, RegKey};

use crate::err::Error;


#[derive(Default)]
pub struct Context {
  do_help: bool,
  svcname: String,
  displayname: Option<String>,
  description: Option<String>,
  deps: Vec<String>
}


pub fn parse(args: Vec<String>) -> Result<Option<Context>, Error> {
  let ctx = Context {
    ..Default::default()
  };

  let mut prsr = arg::Parser::from_args("install-service", args, ctx);

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
      .sopt('d')
      .lopt("description")
      .help(&["Service description."])
      .nargs(arg::Nargs::Count(1), &["DESC"])
      .build(|_spec, ctx: &mut Context, args| {
        ctx.description = Some(args[0].clone());
      })
  )?;

  prsr.add(
    arg::Builder::new()
      .sopt('D')
      .lopt("depends")
      .help(&["Add a dependency."])
      .nargs(arg::Nargs::Count(1), &["SERVICE"])
      .build(|_spec, ctx: &mut Context, args| {
        ctx.deps.push(args[0].clone());
      })
  )?;

  prsr.add(
    arg::Builder::new()
      .sopt('N')
      .lopt("display-name")
      .help(&["Display name."])
      .nargs(arg::Nargs::Count(1), &["TITLE"])
      .build(|_spec, ctx: &mut Context, args| {
        ctx.displayname = Some(args[0].clone());
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

  println!("==> Registering event log source '{}' ..", ctx.svcname);
  eventlog::register(&ctx.svcname)?;

  let manager_access =
    ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
  let service_manager =
    ServiceManager::local_computer(None::<&str>, manager_access)?;

  let service_binary_path = ::std::env::current_exe()?;

  // Generate service launch arguments
  let mut launch_args: Vec<OsString> = Vec::new();

  // Make sure the service launches in service mode
  launch_args.push(OsString::from("run-service"));
  launch_args.push(OsString::from(&ctx.svcname));

  let deps: Vec<ServiceDependency> = ctx
    .deps
    .iter()
    .map(|s| ServiceDependency::Service(OsString::from(s)))
    .collect();

  let displayname = if let Some(dn) = ctx.displayname {
    dn.clone()
  } else {
    ctx.svcname.clone()
  };

  let service_info = ServiceInfo {
    name: OsString::from(&ctx.svcname),
    display_name: OsString::from(displayname),
    service_type: ServiceType::OWN_PROCESS,
    start_type: ServiceStartType::AutoStart,
    error_control: ServiceErrorControl::Normal,
    executable_path: service_binary_path,
    launch_arguments: launch_args,
    dependencies: deps,
    account_name: None, // run as System
    account_password: None
  };
  println!("==> Registering service '{}' ..", ctx.svcname);
  let service = service_manager
    .create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
  if let Some(desc) = ctx.description {
    service.set_description(desc)?;
  } else {
    service.set_description("Service Wrapper Service")?;
  }

  println!("==> Service installation successful");

  let params = create_service_params(&ctx.svcname)?;

  /*
  let curdir = env::current_dir()?;
  let curdir = curdir.to_str().unwrap();
  params.set_value("WorkDir", &curdir)?;
  */

  params.set_value("LogLevel", &"warn")?;

  Ok(())
}


/// Create a Parameters subkey for a service.
pub fn create_service_params(
  service_name: &str
) -> Result<winreg::RegKey, Error> {
  let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
  let services = hklm.open_subkey("SYSTEM\\CurrentControlSet\\Services")?;
  let asrv = services.open_subkey(service_name)?;
  let (subkey, _disp) = asrv.create_subkey("Parameters")?;

  Ok(subkey)
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
