# Cupcake Inspect Command Demo
# Generates: cupcake-inspect.gif
# Run from repo root: just asset inspect

Source tapes/common/settings.tape

Output ../docs/assets/cupcake-inspect.gif

# Setup: go to a project with cupcake initialized
Hide
Type "cd /tmp && rm -rf cupcake-demo && mkdir cupcake-demo && cd cupcake-demo"
Enter
Sleep 300ms
Type "cupcake init --harness claude 2>/dev/null"
Enter
Sleep 1s
Type "clear"
Enter
Sleep 300ms
Show

# Show inspect command with less for scrolling
Type "cupcake inspect | less"
Sleep 500ms
Enter
Sleep 2s

# Scroll down through inspect output
Type "jjjjjjjjjj"
Sleep 800ms
Type "jjjjjjjjjj"
Sleep 800ms
Type "jjjjjjjjjj"
Sleep 800ms
Type "jjjjjjjjjj"
Sleep 800ms

# Scroll back to top
Type "gg"
Sleep 1s

# Exit less
Type "q"
Sleep 500ms

# Also show table format with less
Type "cupcake inspect --table | less"
Sleep 500ms
Enter
Sleep 2s

# Scroll through table
Type "jjjjjjjjjj"
Sleep 800ms
Type "jjjjjjjjjj"
Sleep 800ms

# Back to top
Type "gg"
Sleep 1s

# Exit less
Type "q"
Sleep 500ms
