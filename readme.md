<img align="left" width="80" height="80" src="https://raw.githubusercontent.com/pileghoff/retread/main/icon.svg">

# Retread

Retread is a log file replay and debugging tool designed to streamline the debugging process when only the  a log file is available. It offers an efficient solution for scenarios where reproducing defects is challenging or attaching a debugger is not feasible. By providing the ability to step through log messages and correlate them with the corresponding source code, Retread allows developers to gain crucial insights into the application's behavior.

## How it works

Retread acts as a Debug Adaptor, allowing your editor of choice to connect and issue debug commands such as setting breakpoints in either your source file or log file and stepping though the log messages (both forwards and backwards), and Retread will then inform the editor or the line of source-code corresponding to each log message. Because we utilize the standardized Debug Adaptor Protocol, Retread can be used in most editors and you will be presented with a familiar debug interface.

Mapping log messages to source files is not an exact science. You can provide Retread with a regex to parse the Log files and find any useful meta-data that might be hiding in there, such as filename, line number or function name. This will both speedup the process of matching log messages to source files and make it much more accurate. 

## Setup

Retread uses the standardized [Debug adaptor protocol](https://microsoft.github.io/debug-adapter-protocol/overview), and can be used with any IDE that supports DAP. We have provided a VSCode extension that includes the Retread binary, for getting up and running.

For other systems, please not that we do currently support only support `launch` and not `attach` when initializing the debug adaptor.

When sending the `launch` command, the following configurations options should be provided. These same configurations options should be set in the `launch.json` configuration in VSCode.

### Configuration options

- `log_file`, `string`: Path to the log file you wish to emulate.
- `log_pattern`, `string`: Regex that tells Retread how to dissect each line of the log file. The regex uses named capture groups to analyses the log. The following named groups are supported:
  - `message`: Required. Contains the logged message, without any metadata.
  - `file`: Optional. Contains the path or name of the file where the message was logged.
  - `line`: Optional. Contains the linenumber where the message was logged.
  - `func`: Optional. Contains the name of the function where the message was logged.
- `include`, `Array[string]`: An array of glob patterns, for all the source files to search.
- `exclude`, `Array[string]`: An array of glob patterns, for all the source files to exclude from the search.

Example config:
```json
{
    "log_file": "~/my_log.txt",
    "log_pattern": "\\[(?P<file>\\w+) : (?P<line>\\d+)\\] (?P<message>.*)$",
    "include": ["./linux/**/*.c"],
    "exclude": []
}
```

This configuration will search for all C files in the linux source tree, and match using the following log format:

```
[linux/lib/clz_ctz.c:27] Log message example
[linux/mm/memcontrol.c:5448] Hello world 
[linux/crypto/hmac.c:84] Another log message
...
```