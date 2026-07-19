import re

with open('ui/src/components/AiAgent.tsx', 'r') as f:
    content = f.read()

# Replace border-related rgba(255, 255, 255, x) with var(--border-primary)
content = re.sub(r'1px solid rgba\(255, 255, 255, 0\.[0-9]+\)', '1px solid var(--border-primary)', content)

# Replace background hover/active rgba with var(--bg-hover) or just a generic one
content = re.sub(r'"rgba\(255, 255, 255, 0\.(05|06|08|1)\)"', '"var(--bg-secondary)"', content)

# Replace remaining text rgba(255, 255, 255, x) with var(--text-secondary) or var(--text-muted) depending on opacity
content = re.sub(r'"rgba\(255, 255, 255, 0\.(65|7|75)\)"', '"var(--text-secondary)"', content)
content = re.sub(r'"rgba\(255, 255, 255, 0\.(25|35|4)\)"', '"var(--text-muted)"', content)

# Replace "#fff" with "var(--text-primary)" for text selection
content = re.sub(r'isSelected \? "#fff" :', 'isSelected ? "var(--text-primary)" :', content)
content = re.sub(r'inputVal\.trim\(\) \? "#fff" :', 'inputVal.trim() ? "var(--text-primary)" :', content)

with open('ui/src/components/AiAgent.tsx', 'w') as f:
    f.write(content)
