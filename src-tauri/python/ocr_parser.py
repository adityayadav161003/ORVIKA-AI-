import sys
import json
import traceback
import os

def main():
    try:
        import pytesseract
        from pdf2image import convert_from_path
        from PIL import Image
    except ImportError as e:
        print(json.dumps({"status": "error", "error": f"OCR dependencies missing: {str(e)}"}), flush=True)
        sys.exit(1)

    # On Windows, if Tesseract isn't in PATH, check common installation paths
    if os.name == 'nt':
        common_paths = [
            r"C:\Program Files\Tesseract-OCR\tesseract.exe",
            r"C:\Program Files (x86)\Tesseract-OCR\tesseract.exe",
            os.path.expandvars(r"%LOCALAPPDATA%\Programs\Tesseract-OCR\tesseract.exe")
        ]
        for path in common_paths:
            if os.path.exists(path):
                pytesseract.pytesseract.tesseract_cmd = path
                break

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
            
        try:
            req = json.loads(line)
            file_path = req.get("file_path")
            
            if not file_path:
                print(json.dumps({"status": "error", "error": "file_path is required"}), flush=True)
                continue
                
            if not os.path.exists(file_path):
                print(json.dumps({"status": "error", "error": f"File not found: {file_path}"}), flush=True)
                continue

            ext = os.path.splitext(file_path)[1].lower()
            
            content = ""
            confidence_scores = []
            
            # Check if tesseract binary is functional
            try:
                pytesseract.get_tesseract_version()
            except Exception:
                print(json.dumps({
                    "status": "error", 
                    "error": "Tesseract executable not found or not functional. Please install Tesseract-OCR."
                }), flush=True)
                continue

            if ext == ".pdf":
                # Convert PDF to images
                images = convert_from_path(file_path)
                if not images:
                    print(json.dumps({"status": "error", "error": "Could not convert PDF pages to images"}), flush=True)
                    continue
                    
                for i, img in enumerate(images):
                    # Run OCR on page
                    data = pytesseract.image_to_data(img, output_type=pytesseract.Output.DICT)
                    page_text = pytesseract.image_to_string(img)
                    
                    # Compute average confidence for words that have a confidence score
                    confidences = [int(c) for c in data['conf'] if int(c) != -1]
                    avg_conf = sum(confidences) / len(confidences) if confidences else 0.0
                    
                    content += f"\n--- Page {i+1} ---\n" + page_text
                    confidence_scores.append(avg_conf)
            else:
                # Try loading directly as PIL image (for png, jpg, jpeg, tiff, etc)
                img = Image.open(file_path)
                data = pytesseract.image_to_data(img, output_type=pytesseract.Output.DICT)
                page_text = pytesseract.image_to_string(img)
                
                confidences = [int(c) for c in data['conf'] if int(c) != -1]
                avg_conf = sum(confidences) / len(confidences) if confidences else 0.0
                
                content = page_text
                confidence_scores.append(avg_conf)

            print(json.dumps({
                "status": "success",
                "content": content,
                "confidence_per_page": confidence_scores
            }), flush=True)
            
        except Exception as e:
            err = traceback.format_exc()
            print(json.dumps({
                "status": "error",
                "error": str(e),
                "traceback": err
            }), flush=True)

if __name__ == "__main__":
    main()
