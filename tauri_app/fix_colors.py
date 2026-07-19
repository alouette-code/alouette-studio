import re

with open('ui/src/components/AiAgent.tsx', 'r') as f:
    content = f.read()

# Replace hardcoded dark background for sidebar
content = re.sub(r'variant === "full" \? "var\(--bg-primary\)" : "#18181b"', 'variant === "full" ? "var(--bg-primary)" : "var(--bg-secondary)"', content)

# Replace ternary for text primary
content = re.sub(r'variant === "full" \? "var\(--text-primary\)" : "rgba\(255, 255, 255, 0\.9\)"', '"var(--text-primary)"', content)

# Replace ternary for text secondary
content = re.sub(r'variant === "full" \? "var\(--text-secondary\)" : "rgba\(255, 255, 255, 0\.(45|65)\)"', '"var(--text-secondary)"', content)

# Replace ternary for text muted
content = re.sub(r'variant === "full" \? "var\(--text-muted\)" : "rgba\(255, 255, 255, 0\.35\)"', '"var(--text-muted)"', content)

# Replace hover state hardcoded rgba to text-primary/secondary
content = re.sub(r'e\.currentTarget\.style\.color = "rgba\(255, 255, 255, 0\.(75|8)\)"', 'e.currentTarget.style.color = "var(--text-primary)"', content)
content = re.sub(r'e\.currentTarget\.style\.color = "rgba\(255, 255, 255, 0\.(35|45)\)"', 'e.currentTarget.style.color = "var(--text-secondary)"', content)
content = re.sub(r'color: "rgba\(255, 255, 255, 0\.(45|4)\)"', 'color: "var(--text-secondary)"', content)
content = re.sub(r'color: "rgba\(255, 255, 255, 0\.8\)"', 'color: "var(--text-primary)"', content)
content = re.sub(r'color: "rgba\(255, 255, 255, 0\.35\)"', 'color: "var(--text-muted)"', content)

# Replace ternary for border
content = re.sub(r'variant === "full" \? "1px solid rgba\(255, 255, 255, 0\.15\)" : "1px solid rgba\(255, 255, 255, 0\.08\)"', '"1px solid var(--border-primary)"', content)

with open('ui/src/components/AiAgent.tsx', 'w') as f:
    f.write(content)
