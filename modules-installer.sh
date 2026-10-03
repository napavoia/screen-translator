DETECTION_MODEL="https://ocrs-models.s3-accelerate.amazonaws.com/text-detection.onnx"
RECOGNITION_MODEL="https://ocrs-models.s3-accelerate.amazonaws.com/text-recognition.onnx"

mkdir models
curl "$DETECTION_MODEL" -o models/text-detection.onnx
curl "$RECOGNITION_MODEL" -o models/text-recognition.onnx
