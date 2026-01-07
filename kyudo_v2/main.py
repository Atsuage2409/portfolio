import cv2
from detector import KyudoDetector
from judge import GridJudge
from score_manager import ScoreManager
import config
def main():
    # --- 1. インスタンス生成 ---
    # conf=0.4: テスト用に少し感度を上げておきます
    detector = KyudoDetector() 
    judge = GridJudge()
    manager = ScoreManager()

    # --- 2. カメラ起動 ---
    cap = cv2.VideoCapture(config.video_source) # カメラID (必要に応じて変更)
    cap.set(cv2.CAP_PROP_FRAME_WIDTH, config.frame_width)
    cap.set(cv2.CAP_PROP_FRAME_HEIGHT, config.frame_height)

    print("System Started. Press 'q' to quit.")
    print("【モード】画面全体を5x4のグリッドとして判定します")

    try:
        while True:
            ret, frame = cap.read()
            if not ret: break

            # 画面サイズを取得
            h, w, _ = frame.shape
            
            # 「画面全体」を掲示板のエリアとする
            board_rect = (0, 0, w, h)

            # --- A. 認識 ---
            raw_detections = detector.detect(frame)

            # --- B. 判定 (画面全体を使ってグリッド判定) ---
            scoreboard = judge.update(raw_detections)

            # --- C. GUI更新 ---
            manager.update_gui(scoreboard)

            # 2. 認識結果の枠を描画
            for det in raw_detections:
                x1, y1, x2, y2 = det['box']
                label = det['name']
                
                # 色設定 ("maru"は緑、"batsu"は赤)
                color = (0, 255, 0) if label == "O" else (0, 0, 255)
                
                # 枠線
                cv2.rectangle(frame, (x1, y1), (x2, y2), color, 2)
            
            cv2.imshow("Kyudo Board Reader (Full Screen)", frame)

            if cv2.waitKey(1) & 0xFF == ord('q'):
                break

    finally:
        cap.release()
        cv2.destroyAllWindows()
        manager.close()

if __name__ == "__main__":
    main()