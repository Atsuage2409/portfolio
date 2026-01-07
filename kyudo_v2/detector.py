import cv2
from ultralytics import YOLO
import config

class KyudoDetector:
    def __init__(self):
        print(f"Loading AI Model: {config.model_path}...")
        self.model = YOLO(config.model_path)

    def detect(self, frame):

        # 透明対策（4ch -> 3ch）
        if frame.shape[2] == 4:
            frame = cv2.cvtColor(frame, cv2.COLOR_BGRA2BGR)

        # AI推論 (verbose=Falseでログ抑制)
        results = self.model(frame, conf = config.conf, verbose=False)
        
        raw_detections = []
        
        # 何も検出されなかった場合
        if results[0].boxes is None:
            return raw_detections

        # 検出データを整形
        for box in results[0].boxes:
            cls_id = int(box.cls[0])
            name = self.model.names[cls_id] # "maru" or "batsu"
            x, y, w, h = box.xywh[0] # 中心座標
            x1, y1, x2, y2 = box.xyxy[0] # 描画用座標

            raw_detections.append({
                "name": name,          # AIが見た生の結果
                "box": (int(x1), int(y1), int(x2), int(y2)), # 描画用
                "conf": float(box.conf[0]) # 信頼度
            })
        return raw_detections