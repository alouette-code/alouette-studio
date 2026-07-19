import re

with open('ui/src/index.css', 'r') as f:
    content = f.read()

# Replace variables in index.css for multi-agent
replacements = {
    r'var\(--bg-tertiary,\s*#[0-9a-fA-F]+\)': 'var(--bg-secondary)',
    r'var\(--bg-color,\s*#[0-9a-fA-F]+\)': 'var(--bg-primary)',
    r'var\(--border-color,\s*#[0-9a-fA-F]+\)': 'var(--border-primary)',
    r'var\(--text-color,\s*#[0-9a-fA-F]+\)': 'var(--text-primary)',
    r'var\(--text-muted,\s*#[0-9a-fA-F]+\)': 'var(--text-muted)',
    r'rgba\(255,\s*255,\s*255,\s*0\.05\)': 'var(--bg-hover)',
    r'rgba\(255,\s*255,\s*255,\s*0\.08\)': 'var(--bg-hover)',
    r'rgba\(255,\s*255,\s*255,\s*0\.03\)': 'var(--bg-hover)',
    r'rgba\(255,\s*255,\s*255,\s*0\.1\)': 'var(--bg-hover)'
}

for old, new in replacements.items():
    content = re.sub(old, new, content)

# Also fix color: #ffffff; to color: var(--text-primary); in multi-agent hover states
content = re.sub(r'color:\s*#ffffff;', 'color: var(--text-primary);', content)

with open('ui/src/index.css', 'w') as f:
    f.write(content)
