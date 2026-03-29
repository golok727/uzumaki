## Cant import non-index files from a package wihout import specifiers  if no extension is provided.

this bahavior is not consistent with how we impoer files without extensions it should check imporrt specifiers or a file itself 
```
`Result::unwrap()` on an `Err` value: CoreError(JsBox(JsErrorBox { inner: ImportPrefixMissing { specifier: "react-reconciler/constants", maybe_referrer: Some("file:///C:/Users/Radha/dev/kimi/uzumaki/crates/uzumaki/js/react/reconciler.ts") } }))
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
C:\Users\Radha\dev\kimi\uzumaki\packages\playground:
```