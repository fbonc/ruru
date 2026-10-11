# Architecture

This document defines the conventions for organizing the application. It describes responsibilities and boundaries rather than prescribing a fixed list of crates, components, windows, or features. The implementation can evolve while retaining these rules.

## Responsibilities and dependencies

Organize code around the responsibility it owns. A crate boundary is useful when it makes ownership, dependencies, or an external interface explicit. A module can provide the same separation within a crate when an additional crate would add little value.

| Responsibility | Owns |
| --- | --- |
| Application host | Window lifecycle, framework integration, and translation between UI and core messages |
| UI | Presentation state, interaction handling, rendering, and appearance |
| Core | Application behavior, coordination of operations, and application invariants |
| Configuration and storage | Persistent data formats, loading, validation, saving, migrations, and credential storage |
| Provider adapters | Provider interfaces, protocol handling, discovery, and external model requests |
| Platform adapters | Native events, operating system APIs, and platform resource lifetimes |
| Shared value types | Data exchanged across boundaries |

The host uses core as the interface to application behavior. Provider selection, configuration rules, persistence coordination, and request orchestration belong behind that interface. UI communicates intent and receives results through the host.

Core invokes storage, provider, and platform capabilities as needed. Those capabilities expose their own interfaces and do not depend on UI or the application host. UI should depend on presentation concerns and the value types needed to describe its inputs and outputs.

Some native integration belongs directly to the host. Window focus, visibility, framework callbacks, and events requiring the UI thread follow the ownership and thread requirements of the window system. A dependency should follow the responsibility it serves; adding a forwarding layer through core is useful only when core has a role in the operation.

## State and ownership

Each stateful component owns the data needed for its responsibility. Keep persistent configuration, application runtime state, and presentation state distinct.

- Persistent configuration describes saved preferences and choices.
- Application runtime state describes active operations and the data needed to coordinate them.
- Presentation state describes drafts, navigation, focus, expansion, loading indicators, and other interaction details.

UI may hold a projection of application data for display. Core remains responsible for the application rules that govern that data. A draft and a saved value have different lifetimes and should be represented accordingly.

Keep runtime fields private outside their owning component. Expose inputs and read-only accessors instead of allowing external callers to assign fields. Use visibility scoped to the containing module when sibling implementation files need access.

Shared resources should have an explicit owner and lifetime. Use shared ownership when multiple consumers actually need it. Avoid copying ownership into several coordinators or adding a wrapper whose only purpose is to forward another object's methods.

Serialized configuration remains a data representation. Credential storage is an operation with its own backend and failure modes. Keeping those responsibilities separate allows core to coordinate them without making serialized data own a live storage service.

## State, behavior, and presentation

A component remains one Rust type. Separate implementation blocks by responsibility across files; the split does not require additional controller objects or a trait for every component.

Production code in `state.rs` is declarative. It contains struct and enum definitions, fields, supporting type aliases, and derives. Handwritten implementations and operation bodies belong in the relevant behavior files; test-only code follows the testing rules below.

Constructors, handwritten `Default` implementations, initialization builders, and simple read-only accessors belong in the component's `mod.rs`, or in `lib.rs` at a crate root. Construction that performs substantial work, such as loading stored data, belongs in an operation file.

Behavior lives in input handlers and files named for the operations they implement. Views borrow state and construct presentation. Appearance belongs in style files. Rendering does not mutate state, start application operations, or perform persistence.

Small value types, message definitions, protocol payloads, and stateless rendering or parsing helpers stay with the interface or operation they describe. They do not require a stateful component structure merely because they are represented by a Rust struct.

## Inputs, outputs, and effects

A stateful UI component changes at runtime through its `update` entry point. Inputs include both user actions and results delivered by the host. Successful saves, failures, discovered data, and model changes enter through the same mechanism as other state transitions.

Core receives application intents through `handle_input`. That handler owns dispatch and delegates substantial work to the relevant operation or component. Bootstrap code connects the input and output streams; it does not contain a second dispatcher for selected features.

The usual flow is:

```text
user action
    -> UI input
    -> UI update
    -> UI output
    -> host translation
    -> core input
    -> application operation
    -> core output
    -> host translation
    -> UI input
    -> UI update
```

UI consumes local interactions itself. It emits an output when the host or core must act. Results delivered to UI update its presentation state and should not accidentally trigger the same operation again.

The host translates messages, handles delivery failures, and applies window effects. Core coordinates business operations. Storage owns the mechanics of persistence; adapters own communication with external systems. Keep decisions at the layer that owns the relevant rule.

UI framework tasks, widget operations, and subscriptions can stay within presentation components when they concern presentation behavior. Application operations are represented as intent and routed to core.

## Asynchronous operations and failures

Give asynchronous operations an explicit lifecycle. Define when they start, complete, fail, cancel, and become obsolete. Results from an obsolete operation must not overwrite current state.

Use request identities, cancellation handles, or revisions where the operation needs them. Keep that bookkeeping with the operation's owner and name it for the work it identifies. Distinguish cancellation from discarding retained data when retries or other behavior require different lifetimes.

Apply state changes at the point required by the invariant. For example, cancellation needed to invalidate an active request should occur when the change is accepted, even if delivery of the resulting notification happens later.

Configuration changes must account for persistence failures. The coordinating layer defines when a change becomes effective and how to preserve or restore the previous value if a required step fails. Storage implements the underlying writes and reports their outcomes.

Represent errors where they affect behavior or interaction. UI can retain an error for display; core can retain a load failure when it affects whether editing is allowed. Give retained errors a defined lifetime and clearing condition. Preserve the information callers need to handle the failure, and keep secrets out of logs and debug representations.

## File roles

Use a small, consistent set of file roles. Create each file when the component has that responsibility.

| File | Responsibility |
| --- | --- |
| `mod.rs` / `lib.rs` | Module declarations, exports, message interfaces, construction, and simple accessors |
| `state.rs` | State declarations, fields, type aliases, and derives |
| `update.rs` | UI input dispatch and state transitions |
| `handle_input.rs` | Application input dispatch and coordination |
| `view.rs` | Rendering from borrowed state |
| `style.rs` | Appearance, layout constants, and widget styles |
| `subscription.rs` | Event subscriptions and timers |
| `runtime.rs` | Framework setup, event-loop wiring, and bootstrap integration |
| `adapter.rs` | Implementation of an external capability interface and interaction with that system |
| `protocol.rs` | External wire types, encoding, decoding, and protocol interpretation |
| A responsibility name, such as `capture.rs`, `discovery.rs`, or `navigation.rs` | One distinct operation or feature responsibility |

Choose a standard role name when it applies. Otherwise, name the file for the responsibility it owns. Avoid generic collections such as `utils.rs`, `helpers.rs`, or `operations.rs` that provide no clear rule for what belongs inside them.

Input handlers retain dispatch and delegate substantial operations. Adapter code handles external interaction; protocol code interprets external data. A feature operation coordinates its work using those interfaces. Helpers stay beside the responsibility they support until they have a clear independent purpose or are shared by multiple responsibilities.

Component-specific styles stay with the component. Appearance used by multiple components belongs in shared style code. Share common definitions where they have the same meaning.

## Files and component boundaries

Create a file when the code represents a distinct responsibility with a clear interface and callers. File length alone does not determine a split.

Use a file for a cohesive operation that does not need its own independently owned state or lifecycle. Use a directory when a component owns state and has its own inputs or lifecycle, or when an adapter or other responsibility needs several standard implementation roles. Apply the same conventions within that directory.

Keep related code together. A helper used by one operation belongs beside that operation. Shared code belongs at the narrowest common ownership boundary. Extracting code should make its ownership and callers easier to understand.

Avoid splitting every method into a file or creating empty files to complete a template. A component only needs the roles it actually implements.

## Interfaces and abstraction

Use traits when they describe a meaningful capability boundary, support interchangeable implementations, or enable code that genuinely operates generically. Similar method names or file layouts alone do not require a shared trait.

UI components can follow the same `update` and `view` convention while retaining their own input and output types. A shared UI trait becomes useful when a caller needs to operate generically on those components.

Keep common provider configuration and coordination independent of individual providers. External systems can still require different protocol adapters. Preserve that distinction so adding an adapter does not require duplicating settings workflows or host logic.

Prefer direct ownership and explicit dependencies. Add a service, registry, or wrapper when it owns a responsibility, maintains an invariant, or provides a needed interface. Its name should explain what it owns or does.

## Verification and evolution

Tests belong in a `#[cfg(test)] mod tests` block at the end of the file, after all production code. Tests spanning several modules belong with the operation or component whose contract they exercise. Test-only functions, fixtures, types, and trait or inherent `impl` blocks must be defined inside `mod tests`; do not add test-only methods to production implementation blocks. Shared fixtures may be reused from the owning component's final `mod tests`, with visibility limited to the tests that need them. Tests generally do not have their own files. Test storage and external interactions using controlled implementations where practical.

Verify meaningful behavior: ownership boundaries, cancellation, stale results, failure recovery, serialization, and message delivery. Live-system tests should be explicit about their environment requirements. Follow the repository's required build and verification checks.

Treat these conventions as the default for new code and refactors. When a responsibility needs a new file role or boundary, define its purpose and extraction rule in this document. The architecture should evolve through explicit conventions that future changes can follow.
