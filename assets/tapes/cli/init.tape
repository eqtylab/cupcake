# Cupcake Init Command Demo
# Generates: cupcake-init.gif
# Run from repo root: just asset init

Source tapes/common/settings.tape

Output output/gifs/cupcake-init.gif

# Setup: create a temp directory (hidden from recording)
Hide
Type "cd /tmp && rm -rf cupcake-demo && mkdir cupcake-demo && cd cupcake-demo"
Enter
Sleep 500ms
Type "clear"
Enter
Sleep 300ms
Show

# Show we're in an empty directory
Type "ls -la"
Sleep 300ms
Enter
Sleep 1s

# Initialize cupcake for Claude Code
Type "cupcake init --harness claude"
Sleep 500ms
Enter
Sleep 2s

# Show what was created with less for scrolling
Type "tree -a .cupcake | less"
Sleep 300ms
Enter
Sleep 2s

# Scroll down through tree output
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
