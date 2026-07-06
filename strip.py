import re
with open('original_with_lines.txt', 'r') as f:
    lines = f.readlines()

start_idx = 0
for i, line in enumerate(lines):
    if re.match(r'^[0-9]+: ', line):
        start_idx = i
        break

out_lines = []
for i in range(start_idx, len(lines)):
    line = lines[i]
    if re.match(r'^[0-9]+: ', line):
        out_lines.append(re.sub(r'^[0-9]+: ', '', line))
    else:
        # Stop if we hit the footer
        if "The above content shows the entire, complete file contents" in line:
            break
        else:
            # Maybe a wrapped line? Actually, the original file might just have the lines.
            pass

with open('original.tsx', 'w') as f:
    f.writelines(out_lines)
