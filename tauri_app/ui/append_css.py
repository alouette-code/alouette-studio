css_rules = """

/* Smooth Resizing Overrides */
body.is-resizing * {
    pointer-events: none !important;
}

body.is-resizing .zone-1,
body.is-resizing .zone-2,
body.is-resizing .zone-3,
body.is-resizing .zone-4,
body.is-resizing .zone-5,
body.is-resizing .zone-file-explorer,
body.is-resizing .col-left,
body.is-resizing .col-center,
body.is-resizing .col-right {
    contain: strict !important;
    overflow: hidden !important;
}
"""

with open('src/index.css', 'a') as f:
    f.write(css_rules)

print("Appended CSS rules successfully!")
