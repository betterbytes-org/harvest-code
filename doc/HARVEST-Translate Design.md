# HARVEST-Translate design

`harvest_translate` is HARVEST's C->Rust translation frontend. Given a C project
as input, it oversees the parsing, analysis, lowering, and testing steps needed
to translate the project into Rust. This document describes the high-level
design of `harvest_translate`: its operating principles, and how it integrates
with the rest of the HARVEST ecosystem.

Note that `harvest_translate` is an active work-in-progress; this document
describes the intended design, not the current state of its implementation.

## Components

`harvest_translate` has the following main components:

* **Main Loop:** Calls out to the other components to invoke tools and process
  their results, repeating until done.
* **Tools:** A collection of operations that perform lifting, lowering,
  analysis, testing, or other operations on the IR.
* **Scheduler:** Determines which tool invocations are should be tried next.
* **Diagnostics:** Produces output that HARVEST developers can use to understand
  what happened during a particular `harvest_translate` invocation.
* **IR Storage:** Stores the HarvestIR and manages edits to it.
* **Tool Runner:** Manager threads for each tool invocation, executes the tools,
  and handles tool results (whether they succeed or error).

### Main Loop

Conceptually, the main loop:

1. Fetches desirable tool invocations from the scheduler.
2. Determines whether the tool can be invoked by calling into the IR storage and
   other components.
3. Passes tools that should be invoked to the tool runner for execution.
4. Waits for tools to complete.
5. Repeats until done.

### Tools

`harvest_translate` has many different types of tools. Each tool may be
implemented entirely in `harvest_translate`'s codebase, or may be implemented by
calling out to external programs (such as LLVM tools). Most tools read (a subset
of) the IR then make changes to the IR to extend or improve it in some way.

They may also produce outputs suggesting other tool invocations, for example a
tool that finds some operation ambiguous (should X be a free function or a
method?) might suggest running a tool that categorizes functions. The format of
these suggestions is to be determined.

Tools provide an interface by which the scheduler "evaluates" invoking them.
This interface takes in some arguments about the invocation (which may direct it
to perform its analyses on a particular part of the source, representation, or
provide any other input as to how it should run), and returns information about
which parts of the IR the tool will write. The scheduler uses this interface to
de-conflict tool invocations.

### Scheduler

The scheduler is responsible for telling the main loop which tools it should try
invoking. It keeps a track of suggested tool invocations, and may also have
logic to suggest tool invocations itself.

### Diagnostics

The diagnostics component provides outputs that allow HARVEST developers to
understand what a `harvest_translate` invocation did and to debug its operation.
Its outputs include:

* Why each tool was selected.
* Inputs and outputs of each tool. Ideally, these would allow developers to
  easily reproduce that tool's operation (e.g. if an external binary is invoked,
  its command line arguments should be in the diagnostic output).
* A history of the HARVEST-IR changes. Example use case: suppose a developer
  discovered that a particular IR invariant was broken during execution. They
  should be able to bisect the IR change history to identify which tool broke
  that invariant.

and any other diagnostic output that HARVEST developers feel is useful.

Each of the other components is expected to call into the diagnostic component
to pass the debug data to it.

#### Diagnostics Directory Tree

The diagnostic output is emitted to a directory. It will have at least the
following subdirectories:

* `ir/` Contains all the revisions of the HARVEST-IR. The first revision
  (after the input project is loaded) will be named `0000` (field width to be
  extended as necessary to keep them all the same size). The second revision
  (after the first tool invocation) will be `0001`, etc.
* `steps/` Contains a subdirectory for each tool invocation. The name of each
  subdirectory is the same as the name of the IR revision immediately after that
  tool finishes. As a result, `0000` will be skipped and the subdirectory
  numbers will start at `0001`. Each subdirectory will contain:
  - `start_ir` A symlink to the IR revision the tool was launched with (i.e.
    links to `../../ir/####`).
  - `end_ir` A symlink to the IR revision the tool was completed with.
  - `messages` A file with diagnostic messages produced by that tool invocation
    (`harvest_translate` should provide each tool with something it can
    `writeln!()` to or a similar logging framework).
  - For each external binary invoked, a subdirectory (name TBD) containing:
    * `cmd` Command line arguments for the tool.
    * `stdout` The program's standard output and error.
    * `stdin` Data fed to the program's standard input.

## Concurrency Model

When a tool is invoked, it is provided with a read-only snapshot of the IR, as
well as read-write copies of the parts of the IR that it said it might write.
The scheduler tracks which parts of the IR are in use (that is, a
currently-running tool invocation might write that part of the IR), and will not
concurrently invoke tools if those tools write the same part of the IR.

## "Maybe" Features

These features may or may not be worth the implementation effort:

Resumability: The ability to restart `harvest_translate` from an intermediate
state of a previous invocation, possibly with different code or tools (e.g. to
test how a tool change will impact the translation results). This seems very
useful for development, but requires the IR (and any other important state) to
be deserializable.
