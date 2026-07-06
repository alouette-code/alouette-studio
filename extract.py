import json

with open('/home/nhatanh/snap/antigravity/5/.gemini/antigravity/brain/297d47b9-ae3b-4547-981c-da4241f8736d/.system_generated/logs/transcript_full.jsonl') as f:
    for line in f:
        data = json.loads(line)
        content_str = str(data.get('content', ''))
        if 'File Path:' in content_str and 'DockerManager.tsx' in content_str and 'The following code has been modified' in content_str:
            if isinstance(data.get('content'), list):
                out = data['content'][0].get('text', '')
            else:
                out = data['content']
            with open('original_with_lines.txt', 'w') as outf:
                outf.write(out)
            break
