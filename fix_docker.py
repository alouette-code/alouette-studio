with open('/home/nhatanh/projet/alouette_studio/tauri_app/ui/src/components/DockerManager.tsx', 'r') as f:
    content = f.read()

content = content.replace('''const getC = (c: any) => ({
  id: c.Id || c.id,
  names: c.Names || c.names,
  state: c.State || c.state,
  image: c.Image || c.image
});''', '''const getC = (c: any) => {
  if (!c) return { id: '', names: [], state: '', image: '' };
  return {
    id: c.Id || c.id,
    names: c.Names || c.names,
    state: c.State || c.state,
    image: c.Image || c.image
  };
};''')

# Now fix the places where I broke it with sed
content = content.replace('containers.find(c => c.id === selectedId)', 'containers.find(c => getC(c).id === selectedId)')
content = content.replace('const isRunning = c.state === "running";', 'const isRunning = getC(c).state === "running";')
content = content.replace('const name = c.names?.[0]?.replace("/", "") || (c.id ? c.id.substring(0,8) : "Unknown");', 'const name = getC(c).names?.[0]?.replace("/", "") || (getC(c).id ? getC(c).id.substring(0,8) : "Unknown");')

# Let's ensure currentContainer uses getC correctly everywhere
# Actually, the sed command didn't undo `getC(currentContainer).id` because the sed only replaced `getC(c).id`.
# Wait, `getC(currentContainer).id` is still there.

with open('/home/nhatanh/projet/alouette_studio/tauri_app/ui/src/components/DockerManager.tsx', 'w') as f:
    f.write(content)
