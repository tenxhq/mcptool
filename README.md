**mcptool is under heavy development and not yet ready for use - check back later!**


# mcptool

A versatile command‑line utility for connecting to, testing, and probing
**MCP** servers from your terminal.

**mcptool** aims to provide a unified Swiss‑army‑knife for developers,
administrators, and CI pipelines that interact with servers speaking the
**MCP** protocol.

---

## Key Features

* **One‑liner connections** – quickly open an interactive MCP session without
  extra setup.
* **Health & latency checks** – measure round‑trip times with a simple `mcptool
  mcp ping`.
* **Capability discovery** – automatically probe which optional commands a
  server supports.
* **Script‑friendly output** – machine‑readable JSON and quiet modes for
  automation.

## Quick Start

```bash
cargo install mcptool
```

## Usage

After installation the binary is available as `mcptool`. Use `--help` on any sub‑command for detailed flags.


### Target Specification

Every sub‑command that expects a *target* accepts a TCP endpoint, HTTP/HTTPS endpoint,
a local command to be spawned in **stdio** mode, or a stored authentication entry.

| Variant                      | Syntax                    | What Happens                                                                                                        |
| ---------------------------- | ------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| **Implicit TCP** *(default)* | `host[:port]`             | Connects via TCP. If no port is given, the command‑specific default applies.                                        |
| **Explicit TCP**             | `tcp://host[:port]`       | Same as above but unambiguous when the host could contain a scheme prefix.                                          |
| **HTTP**                     | `http://host[:port]`      | Connects via HTTP. If no port is given, defaults to port 80.                                                       |
| **HTTPS**                    | `https://host[:port]`     | Connects via HTTPS. If no port is given, defaults to port 443.                                                     |
| **Stdio Command**            | `cmd://<program> [args…]` | Spawns the program locally and speaks MCP over its STDIN/STDOUT pipes. Use quotes when the command contains spaces. |
| **Authentication**           | `auth://<name>`           | Uses a stored authentication entry (see Authentication section below).                                              |

> **Example targets**
>
> * `api.acme.ai` (TCP, default port)
> * `tcp://api.acme.ai:7780` (TCP, port 7780)
> * `http://api.acme.ai` (HTTP, port 80)
> * `https://api.acme.ai:8443` (HTTPS, port 8443)
> * `"cmd://./my‑stdio‑server --some --argument"` (local process)
> * `auth://github` (stored authentication entry)

### Global Commands (run from your shell)

| Command                                      | Purpose                                                                                                                                                                                                                 |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `mcptool connect <target> [--script <file>]` | Connect to the target. Without **`--script`** you drop into an interactive prompt (`>`). With **`--script`** mcptool reads one sub‑command per line from *file*, executes them sequentially, prints results, and exits. |
| `mcptool proxy <target> --log-file <file>`   | Transparently open a stdio transport, and proxy all traffic to target, recording it to *file*.                                                                                                                    |
| `mcptool testserver [--stdio] [--tcp] [--port <port>]` | Run a test MCP server with verbose logging. Use `--stdio` for stdio transport, `--tcp` for TCP transport, or default HTTP on specified port. |
| `mcptool version`                            | Display the mcptool build version & linked MCP revision.                                                                                                                                                                |
| `mcptool help [sub-command]`                 | Show contextual help for any command.                                                                                                                                                                                   |

### Global Options

| Option                                       | Purpose                                                                                                                                                                                                                 |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `--json`                                     | Output results in JSON format                                                                                                                                                                                          |
| `--logs <LEVEL>`                             | Enable logging with specified level (debug, info, notice, warning, error, critical, alert, emergency)                                                                                                                   |
| `--color`                                    | Force color output                                                                                                                                                                                                      |
| `--no-color`                                 | Disable color output                                                                                                                                                                                                    |
| `--quiet`                                    | Suppress all output including JSON output                                                                                                                                                                               |

### MCP Commands (usable inside the prompt *or* from the shell with a `<target>`)

When you are **inside the prompt**, type these commands **without** the `mcptool` prefix and without a target. From the regular shell, prefix them with `mcptool mcp` and provide a `<target>`.

| Prompt form                                   | Shell form                                                     | Purpose                                                                                                                       |
| --------------------------------------------- | -------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `ping`                                        | `mcptool mcp ping <target>`                                    | Measure round‑trip latency.                                                                                                   |
| `init`                                        | `mcptool mcp init <target>`                                    | Initialize connection and display server information.                                                                         |
| `listtools`                                   | `mcptool mcp listtools <target>`                               | List all MCP tools (`tools/list`).                                                                                            |
| `listprompts`                                 | `mcptool mcp listprompts <target>`                             | List predefined prompt templates (`prompts/list`).                                                                            |
| `listresources`                               | `mcptool mcp listresources <target>`                           | List server resources such as databases or file trees (`resources/list`).                                                     |
| `listresourcetemplates`                       | `mcptool mcp listresourcetemplates <target>`                   | List resource templates available for instantiation.                                                                          |
| `setlevel <level>`                            | `mcptool mcp setlevel <target> <level>`                        | Set the logging level on the MCP server.                                                                                     |
| `calltool <tool> [options]`                   | `mcptool mcp calltool <target> <tool> [options]`              | Invoke a tool with arguments. Options: `--arg key=value`, `--interactive`, `--json`                                         |
| `readresource <uri>`                          | `mcptool mcp readresource <target> <uri>`                      | Read a resource by URI.                                                                                                       |
| `getprompt <name> [--arg key=value]`          | `mcptool mcp getprompt <target> <name> [--arg key=value]`      | Get a prompt by name with optional arguments.                                                                                |
| `subscriberesource <uri>`                     | `mcptool mcp subscriberesource <target> <uri>`                 | Subscribe to resource update notifications.                                                                                   |
| `unsubscriberesource <uri>`                   | `mcptool mcp unsubscriberesource <target> <uri>`               | Unsubscribe from resource update notifications.                                                                               |
| `complete <reference> <argument>`             | `mcptool mcp complete <target> <reference> <argument>`         | Get completion suggestions for prompt or resource arguments.                                                                  |

### Interactive Prompt & Script Mode

Once connected **without `--script`**, you can run any sub‑command without specifying the target again, just as you would on the normal command line.

If you **provide `--script mysession.mcp`**, the file is read line‑by‑line and each line is dispatched exactly as if you had typed it at the prompt. After the last line executes the connection closes automatically.

```text
$ mcptool connect api.acme.ai
Connected to api.acme.ai (tcp, proto‑rev 9)
> ping
107 ms
> listtools
summarize      translate      moderate
> calltool summarize --arg text="Hello world"
{"summary":"Hello 🌍"}
> exit
```

### Authentication

Mcptool supports OAuth authentication for HTTP/HTTPS endpoints. Authentication entries can be managed using the `mcptool auth` commands:

| Command                                      | Purpose                                                                                                                                |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| `mcptool auth add <name> [options]`          | Add a new OAuth authentication entry with interactive setup                                                                            |
| `mcptool auth list`                          | List all stored authentication entries                                                                                                 |
| `mcptool auth remove <name>`                 | Remove an authentication entry                                                                                                         |
| `mcptool auth renew <name>`                  | Renew the access token using the refresh token                                                                                         |

Once an authentication entry is stored, you can use it with any MCP command by using the `auth://` target syntax:

```bash
# Add GitHub authentication
mcptool auth add github

# Use the stored authentication
mcptool mcp ping auth://github
mcptool mcp listtools auth://github
mcptool connect auth://github
```

### Examples

```bash
# Check latency to a remote MCP server (implicit TCP)
mcptool mcp ping api.acme.ai

# Check latency to a local stdio server
mcptool mcp ping "cmd://./my‑stdio‑server --some --argument"

# Use stored authentication (e.g., for GitHub Copilot)
mcptool mcp ping auth://github

# List tools using authentication
mcptool mcp listtools auth://github

# Connect via TCP on a non‑default port and immediately run a status query, then exit
mcptool calltool tcp://dev.acme.ai:7780 -- status

# Execute a tool with arguments on a local stdio endpoint
mcptool mcp calltool "cmd://./my‑stdio‑server --some --argument" summarize --arg text="Hello world"

# Use interactive mode to be prompted for each parameter
mcptool mcp calltool api.acme.ai chat.complete --interactive

# Pass a complex JSON payload via STDIN
echo '{"text": "Hello world", "model": "gpt-4"}' | mcptool mcp calltool api.acme.ai chat.complete --json

# Run a scripted sequence without entering the prompt
mcptool connect api.acme.ai --script mysession.mcp
```
