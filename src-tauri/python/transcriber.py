import sys
import json
import traceback
import os

def main():
    try:
        from faster_whisper import WhisperModel
    except ImportError:
        print(json.dumps({"status": "error", "error": "faster-whisper package not installed in venv"}), flush=True)
        sys.exit(1)

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
            
        try:
            req = json.loads(line)
            file_path = req.get("file_path")
            model_size = req.get("model_size", "base")
            use_gpu = req.get("use_gpu", True)
            
            if not file_path:
                print(json.dumps({"status": "error", "error": "file_path is required"}), flush=True)
                continue
                
            if not os.path.exists(file_path):
                print(json.dumps({"status": "error", "error": f"File not found: {file_path}"}), flush=True)
                continue

            # Determine device and compute type
            # Standard Faster-Whisper on CPU uses int8 or float32. GPU uses float16 or int8_float16.
            # To ensure it runs on consumer machines, we fall back to CPU if no GPU.
            # On Windows, cuda detection depends on having nvidia drivers and appropriate libraries.
            device = "cpu"
            compute_type = "float32"
            
            if use_gpu:
                # We can check if CUDA is available or let faster_whisper handle it
                # If CUDA fails, we fallback to cpu
                try:
                    # Let's try loading with cuda first if user wants GPU
                    model = WhisperModel(model_size, device="cuda", compute_type="float16")
                    device = "cuda"
                    compute_type = "float16"
                except Exception:
                    # Fallback to cpu
                    model = WhisperModel(model_size, device="cpu", compute_type="float32")
            else:
                model = WhisperModel(model_size, device="cpu", compute_type="float32")
                
            segments, info = model.transcribe(
                file_path,
                beam_size=5,
                word_timestamps=False
            )
            
            segments_list = []
            for segment in segments:
                segments_list.append({
                    "start": segment.start,
                    "end": segment.end,
                    "text": segment.text.strip()
                })
                
            print(json.dumps({
                "status": "success",
                "segments": segments_list,
                "language": info.language,
                "duration": info.duration,
                "device": device
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
