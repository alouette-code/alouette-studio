import urllib.request
import zipfile
import io
import os
import stat

url = "https://storage.googleapis.com/chrome-for-testing-public/127.0.6533.72/linux64/chrome-linux64.zip"
print(f"Downloading {url}...")
try:
    response = urllib.request.urlopen(url)
    with zipfile.ZipFile(io.BytesIO(response.read())) as zip_ref:
        zip_ref.extractall("/home/nhatanh/projet/alouette_studio/chrome_extract")
    
    # Rename directory
    os.rename("/home/nhatanh/projet/alouette_studio/chrome_extract/chrome-linux64", "/home/nhatanh/projet/alouette_studio/chrome")
    os.rmdir("/home/nhatanh/projet/alouette_studio/chrome_extract")
    
    # Make executable
    exe_path = "/home/nhatanh/projet/alouette_studio/chrome/chrome"
    st = os.stat(exe_path)
    os.chmod(exe_path, st.st_mode | stat.S_IEXEC)
    print("Successfully downloaded and extracted Chrome!")
except Exception as e:
    print(f"Error: {e}")
