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
| `mcptool version`                            | Display the mcptool build version & linked MCP revision.                                                                                                                                                                |
| `mcptool help [sub-command]`                 | Show contextual help for any command.                                                                                                                                                                                   |


### MCP Commands (usable inside the prompt *or* from the shell with a `<target>`)

When you are **inside the prompt**, type these commands **without** the `mcptool` prefix and without a target. From the regular shell, prefix them with `mcptool mcp` and provide a `<target>`.

| Prompt form                                   | Shell form                                                     | Purpose                                                                                                                       |
| --------------------------------------------- | -------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `ping`                                        | `mcptool mcp ping <target>`                                    | Measure round‑trip latency.                                                                                                   |
| `status`                                      | `mcptool status <target>`                                      | Fetch server status/handshake info.                                                                                           |
| `probe`                                       | `mcptool probe <target>`                                       | Run all discovery calls (`listtools`, `listprompts`, `listresources`, `listresourcetemplates`) and print a complete overview. |
| `listtools`                                   | `mcptool mcp listtools <target>`                               | List all MCP tools (`tools/list`).                                                                                            |
| `listprompts`                                 | `mcptool listprompts <target>`                                 | List predefined prompt templates (`prompts/list`).                                                                            |
| `listresources`                               | `mcptool listresources <target>`                               | List server resources such as databases or file trees (`resources/list`).                                                     |
| `listresourcetemplates`                       | `mcptool listresourcetemplates <target>`                       | List resource templates available for instantiation.                                                                          |
| `calltool -- <entity> [args…] [--stdin-json]` | `mcptool calltool <target> -- <entity> [args…] [--stdin-json]` | Invoke a tool, prompt, or resource. Use **`--stdin-json`** to pass a JSON object via STDIN.                                   |

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
> calltool summarize --text "Hello world"
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
mcptool calltool "cmd://./my‑stdio‑server --some --argument" -- summarize --text "Hello world"

# Pass a complex JSON payload via STDIN
cat complex_args.json | mcptool calltool api.acme.ai -- chat.complete --stdin-json

# Run a scripted sequence without entering the prompt
mcptool connect api.acme.ai --script mysession.mcp --quiet | mcptool calltool api.acme.ai -- chat.complete --stdin-json
```
