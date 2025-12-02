# Cupcake Verify Command Demo
# Generates: cupcake-verify.gif
# Run from repo root: just asset verify

Source tapes/common/settings.tape

Output ../docs/assets/cupcake-verify.gif

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

# Run verify command with less for scrolling
Type "cupcake verify --harness claude | less"
Sleep 500ms
Enter
Sleep 2s

# Scroll down through output
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
