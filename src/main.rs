mod err;
mod install;
mod service;
mod uninstall;

use qargparser as arg;

use err::Error;

#[derive(Default)]
pub struct Context {
  do_help: bool,
  do_version: bool,
  cmd: String
}

pub fn parse() -> Result<Option<(Context, Vec<String>)>, Error> {
  let ctx = Context {
    ..Default::default()
  };

  //
  // Set up parser
  //
  let mut prsr = arg::Parser::from_env(ctx);

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
      .exit(true)
      .sopt('V')
      .lopt("version")
      .help(&["Show version and exit."])
      .build(|_spec, ctx: &mut Context, _args| {
        ctx.do_version = true;
      })
  )?;

  prsr.add(
    arg::Builder::new()
      .name("subcmd")
      .required(true)
      .exit(true)
      .help(&[
        "Specify subcommand to run.",
        "Recognized subcommands: install-service, uninstall-service",
        "Pass --help to subcommand for more information."
      ])
      .nargs(arg::Nargs::Count(1), &["CMD"])
      .build(|_spec, ctx: &mut Context, args| {
        ctx.cmd = args[0].clone();
      })
  )?;

  prsr.parse()?;


  if prsr.get_ctx().do_help == true {
    prsr.usage(&mut std::io::stdout());
    return Ok(None);
  }

  if prsr.get_ctx().do_version == true {
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    println!("svcupdate {}", VERSION);
    return Ok(None);
  }

  let remain = prsr.get_remaining_args();

  Ok(Some((prsr.into_ctx(), remain)))
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
  let (ctx, args) = match parse()? {
    Some((ctx, args)) => (ctx, args),
    None => return Ok(())
  };

  match ctx.cmd.as_ref() {
    "install-service" => {
      install::run(args)?;
    }
    "uninstall-service" => {
      uninstall::run(args)?;
    }
    "run-service" => {
      service::run()?;
    }
    _ => {
      return Err(Box::new(Error::ArgParser(
        "Invalid subcommand".to_string()
      )));
    }
  }

  Ok(())
}

// vim: set ft=rust et sw=2 ts=2 sts=2 cinoptions=2 tw=79 :
