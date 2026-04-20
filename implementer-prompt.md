You are an expert Rust engineer. Your task is to execute the following plan:

### Task 2: Update `Translator` to use the trait

**Files:**
- Modify: `src/translator.rs`

- [ ] **Step 1: Change `Translator` struct field**

Change `pub db: Arc<DictionaryDatabase<'a>>` to `pub db: Arc<dyn DictionaryService>`.

- [ ] **Step 2: Fix compilation errors**

Since we are moving from a concrete type to a trait object, you may need to adjust the `Translator::new` and `init` functions to accept the trait object.

- [ ] **Step 3: Commit**

---

Context:
- Project: Yomichan RS
- You are working in an isolated git worktree.
- The DictionaryService trait is already defined and implemented by DictionaryDatabase.
- Your goal is to update the Translator struct in src/translator.rs to use this trait.

Instructions:
1. Update Translator::db to use Arc<dyn DictionaryService>.
2. Update Translator::new and any associated constructor logic.
3. Fix all resulting compilation errors.
4. Ensure the project builds successfully.
5. If the task is DONE, report DONE.
