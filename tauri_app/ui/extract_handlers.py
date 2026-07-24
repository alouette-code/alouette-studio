import re

with open('src/App.tsx', 'r') as f:
    content = f.read()

# We look for all handlers starting with "const handle...ResizeStart = (e: React.MouseEvent) => {"
handlers = re.findall(r'(const handle\w+ResizeStart\s*=\s*\(e: React\.MouseEvent\) => \{.*?\n  \};)', content, re.DOTALL)

for i, h in enumerate(handlers):
    print(f"--- Handler {i} ---")
    print(h)
