# TODO - CacheStore LRU Eviction Fix

- [ ] Implement directory-aware size calculation in `crates/core/src/cache/store.rs`
- [ ] Add LRU eviction triggered by `put()` when `current_size + new_entry_size > max_size`
- [ ] Scan cache files, sort by last access timestamp (fallback to modified), delete oldest until enough space
- [ ] Update access semantics on `get()` so reads affect LRU ordering
- [ ] Add/extend unit tests validating aggregate size stays within limit after evictions
- [x] Run `cargo test` to verify correctness


