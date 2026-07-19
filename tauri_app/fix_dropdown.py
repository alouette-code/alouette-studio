import re

with open('ui/src/components/AiAgent.tsx', 'r') as f:
    content = f.read()

# Replace hardcoded dark backgrounds with var(--bg-primary) or var(--bg-secondary)
content = re.sub(r'background: "#242424"', 'background: "var(--bg-primary)"', content)

# Replace #fff in dropdowns with var(--text-primary)
content = re.sub(r'color: "#fff",\n\s*fontSize: "11px"', 'color: "var(--text-primary)",\n                          fontSize: "11px"', content)

# There is a hover state that turns to #fff:
content = re.sub(r'e\.currentTarget\.style\.color = "#fff"', 'e.currentTarget.style.color = "var(--text-primary)"', content)

# Also check for #1a1a1f
content = re.sub(r'background: "#1a1a1f"', 'background: "var(--bg-primary)"', content)

# Also text color in tooltip "rgba(255,255,255,0.9)"
content = re.sub(r'"rgba\(255,255,255,0\.9\)"', '"var(--text-primary)"', content)
content = re.sub(r'"rgba\(255,255,255,0\.7\)"', '"var(--text-secondary)"', content)
content = re.sub(r'"rgba\(255,255,255,0\.6\)"', '"var(--text-secondary)"', content)
content = re.sub(r'"rgba\(255,255,255,0\.3\)"', '"var(--text-muted)"', content)
content = re.sub(r'"rgba\(255,255,255,0\.12\)"', '"var(--border-primary)"', content)
content = re.sub(r'"rgba\(255,255,255,0\.06\)"', '"var(--border-primary)"', content)

with open('ui/src/components/AiAgent.tsx', 'w') as f:
    f.write(content)
