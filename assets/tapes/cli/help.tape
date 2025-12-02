# Cupcake CLI Help Demo
# Generates: cupcake-help.gif
# Run from repo root: just asset help

Source tapes/common/settings.tape

Output output/gifs/cupcake-help.gif

# Show the main help with less for scrolling
Type "cupcake --help | less"
Sleep 500ms
Enter
Sleep 2s

# Scroll down through help
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
