import re

with open('ui/src/components/AiAgent.tsx', 'r') as f:
    content = f.read()

# Modify the form style to remove background, border, boxShadow, and adjust padding
old_style = """              background: variant === "full" ? "var(--bg-primary)" : "var(--bg-secondary)",
              border: "1px solid var(--border-primary)",
              borderRadius: variant === "full" ? "16px" : "24px",
              padding: variant === "full" ? "14px 20px" : "12px 18px",
              gap: "8px",
              position: "relative",
              boxShadow: variant === "full" ? "0 4px 20px rgba(0, 0, 0, 0.4)" : "0 10px 30px -10px rgba(0, 0, 0, 0.7)","""

new_style = """              background: "transparent",
              border: "none",
              padding: "12px 0",
              gap: "8px",
              position: "relative",
              boxShadow: "none",
              borderTop: "1px solid var(--border-primary)", /* Add a top border instead to separate from chat history */"""

content = content.replace(old_style, new_style)

with open('ui/src/components/AiAgent.tsx', 'w') as f:
    f.write(content)

