with open("App.tsx", "r", encoding="utf-8") as f:
    lines = f.readlines()

start_idx = -1
end_idx = -1

for i, line in enumerate(lines):
    if "// 2. Right Sidebar Width Resize Handler" in line and start_idx == -1:
        # Find the SECOND occurrence because the FIRST is the good one we want to keep?
        pass

# Let's just find the index of the string exactly
content = "".join(lines)
idx1 = content.find("// 2. Right Sidebar Width Resize Handler")
idx2 = content.find("// 2. Right Sidebar Width Resize Handler", idx1 + 1)

if idx2 != -1:
    # Find the end of the 5th handler from idx2
    end_str = "window.addEventListener(\"mouseup\", handleMouseUp);\n  };\n"
    end_idx = content.find(end_str, idx2)
    
    if end_idx != -1:
        end_idx += len(end_str)
        # Delete from idx2 to end_idx
        new_content = content[:idx2] + content[end_idx:]
        with open("App.tsx", "w", encoding="utf-8") as f:
            f.write(new_content)
        print("Duplicates removed.")
    else:
        print("End string not found.")
else:
    print("Second occurrence not found.")
