# "Minimal" service example

This project exists only to facilitate investigation of a potential issue when
running `duct` in a Windows service.


## Installation

Build `svcexample.exe` using `cargo`:

```
PS> cargo build --release
```

Copy the target binary `svcexample.exe` to the target system and install it
(in an elevated command line shell) using:

```
PS> .\svcexample.exe install-service myservice
```


### Debugging tools

_svcexample_ can be built to wait for a debugger to attach just before duct is
used.  This can be enabled using the `dbgtools-win` feature:

```
PS> cargo build --release --features dbgtools-win
```


## Running/Using

Once the service has successfully been installed, make sure there's a
directory called `C:\Temp` in the filesystem, and within it create a file
called `service.ps1` containing:

```
Write-Host "Hello from script"
```

When the service is started (either using the Service manager, or using
`Start-Service myservice` in powershell), it will load and then terminate
pretty quickly.  (If started from the GUI Service manager a dialog will pop up
saying that the service self-terminated .. which is true, and very obvious).

The idea is that the output of the `service.ps1` script (i.e.
`Hello from script`) will be written to `C:\Temp\service.log`, but it appears
to fail.  (`service.log` is created, but no data is written to it).

The Event Viewer (under the `Application`) can be used to inspect the log
messages generated in the service.


## Uninstallation

Run, in an elevated command line shell:

```
PS> .\svcexample.exe uninstall-service myservuce
```

