# Cupcake Trust Command Demo
# Generates: cupcake-trust.gif
# Run from repo root: just asset trust

Source tapes/common/settings.tape

Output output/gifs/cupcake-trust.gif

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

# Show trust help with less for scrolling
Type "cupcake trust --help | less"
Sleep 500ms
Enter
Sleep 1.5s

# Scroll through help
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

# Initialize trust
Type "cupcake trust init"
Sleep 500ms
Enter
Sleep 1.5s

# List trusted items with less for scrolling
Type "cupcake trust list | less"
Sleep 500ms
Enter
Sleep 1.5s

# Scroll through trust list
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
