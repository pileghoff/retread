# Retread

Retread is a log file replay and debugging tool designed to streamline the debugging process when only the  a log file is available. It offers an efficient solution for scenarios where reproducing defects is challenging or attaching a debugger is not feasible. By providing the ability to step through log messages and correlate them with the corresponding source code, Retread allows developers to gain crucial insights into the application's behavior.

## How it works

Retread acts as a Debug Adaptor, allowing your editor of choice to connect and issue debug commands such as setting breakpoints in either your source file or log file and stepping though the log messages (both forwards and backwards), and Retread will then inform the editor or the line of source-code corresponding to each log message. Because we utilize the standardized Debug Adaptor Protocol, Retread can be used in most editors and you will be presented with a familiar debug interface.

Mapping log messages to source files is not an exact science. You can provide Retread with a regex to parse the Log files and find any useful meta-data that might be hiding in there, such as filename, line number or function name. This will both speedup the process of matching log messages to source files and make it much more accurate. 

## Setup

### Vscode

### Other
todo