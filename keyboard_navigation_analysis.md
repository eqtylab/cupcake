# Keyboard Navigation Analysis - Cupcake TUI

## Key Mappings Matrix

| Screen | Space | Enter | Up/Down | Other Keys | Notes |
|--------|-------|-------|---------|------------|-------|
| **Landing** | ✅ Continue to next screen | ❌ No action | ✅ Toggle between auto-discovery and manual mode | Esc/Q: Exit | Space is primary action |
| **Discovery** | ✅ Continue (if files selected) | ✅ Toggle file selection | ✅ Navigate file list | Esc/Q: Exit | Both Space and Enter have actions |
| **Extraction** | ❌ No action | ✅ Continue (when complete) | ❌ No navigation | Esc/Q: Exit | Enter only when all tasks complete |
| **Review** | ✅ Continue to compilation | ✅ Toggle rule selection | ✅ Navigate rule list | e: Edit rule, Esc/Q: Exit | Both Space and Enter have actions |
| **Compilation** | ❌ No action | ❌ No action (implicit continue when done) | ❌ No navigation | l: Toggle logs, v: Verbose, r: Retry | No primary navigation keys |
| **Success** | ❌ No action | ✅ Exit application | ❌ No navigation | Esc/Q: Exit | Enter is exit action |

## Key Usage Patterns

### Space Key
- **Primary usage**: Continue/proceed to next screen
- **Used in**: Landing (start), Discovery (continue), Review (continue)
- **NOT used in**: Extraction, Compilation, Success
- **Inconsistency**: In Discovery and Review, Space continues while Enter selects items

### Enter Key
- **Primary usage**: Select/toggle items OR continue/confirm
- **Used in**: Discovery (toggle), Extraction (continue), Review (toggle), Success (exit)
- **NOT used in**: Landing, Compilation
- **Inconsistency**: Mixed use - sometimes selection, sometimes continuation

### Arrow Keys (Up/Down)
- **Primary usage**: Navigation between items
- **Used in**: Landing (mode selection), Discovery (file list), Review (rule list)
- **NOT used in**: Extraction, Compilation, Success
- **Consistent**: Always used for navigation when available

## Inconsistencies Found

1. **Space vs Enter for continuation**:
   - Landing: Space continues
   - Extraction: Enter continues
   - Discovery/Review: Space continues, Enter selects

2. **Modal behavior in Discovery**:
   - When custom prompt modal is shown, Enter applies and continues
   - This creates a third pattern where Enter both applies and continues

3. **Compilation screen has no primary keys**:
   - No Space or Enter actions
   - Only special keys (l, v, r) work
   - Transition happens automatically

4. **Exit behavior**:
   - Success screen: Enter exits
   - All other screens: Esc/Q exits
   - This makes Success screen unique

## Context-Dependent Behaviors

### Discovery Screen
- **Normal mode**: 
  - Enter: Toggle file selection
  - Space: Continue to next screen
- **Modal mode** (custom prompt):
  - Enter: Apply and continue
  - Esc: Cancel modal
  - s: Skip and continue

### Review Screen
- **Normal mode**:
  - Enter: Toggle rule selection
  - Space: Continue to next screen
- **Search mode**:
  - Enter/Esc: Exit search mode
- **Edit modal** (if implemented):
  - Ctrl+Enter: Save
  - Esc: Cancel

## Recommendations for Consistency

1. **Standardize continuation key**: Use Space consistently for "continue to next screen"
2. **Standardize selection key**: Use Enter consistently for "select/toggle current item"
3. **Add Enter action to Landing**: Make Enter also continue (in addition to Space)
4. **Fix Compilation screen**: Add Enter to continue when complete (currently automatic)
5. **Standardize exit**: Consider making Success screen use Esc/Q like others, with Enter as secondary option

## Help Text Analysis

The help text generally reflects the actual behavior, but there are some observations:
- Landing: Shows "Press Space to begin" - accurate
- Discovery: Shows both Enter (Select) and Space (Continue) - accurate
- Extraction: Shows "Press Enter to continue when all files complete" - accurate
- Review: Shows both Enter (Select) and Space (Continue) - accurate
- Compilation: Only shows Esc/Q to exit - missing the automatic progression
- Success: Shows "Press Enter to exit" - accurate but inconsistent with other screens