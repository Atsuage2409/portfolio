import sys
import cv2
from detector import KyudoDetector
from judge import GridJudge
from score_manager import ScoreManager
import config

def process_frame(frame, detector, judge, manager):
    h, w, _ = frame.shape
    raw_detections = detector.detect(frame)
    scoreboard = judge.update(raw_detections)
    manager.update_gui(scoreboard)

    for det in raw_detections:
        x1, y1, x2, y2 = det['box']
        label = det['name']
        color = (0, 255, 0) if label == "O" else (0, 0, 255)
        cv2.rectangle(frame, (x1, y1), (x2, y2), color, 2)

    return frame

def main():
    # --- 1. インスタンス生成 ---
    detector = KyudoDetector()
    judge = GridJudge()
    manager = ScoreManager()

    # --- 画像ファイルが引数で指定された場合 ---
    if len(sys.argv) > 1:
        image_path = sys.argv[1]
        frame = cv2.imread(image_path)
        if frame is None:
            print(f"Error: 画像を読み込めませんでした: {image_path}")
            sys.exit(1)

        print(f"画像モード: {image_path}")
        frame = process_frame(frame, detector, judge, manager)
        cv2.imshow("Kyudo Board Reader", frame)
        print("キーを押すと終了します")
        cv2.waitKey(0)
        cv2.destroyAllWindows()
        manager.close()
        return

    # --- 2. カメラ起動 ---
    cap = cv2.VideoCapture(config.video_source)
    cap.set(cv2.CAP_PROP_FRAME_WIDTH, config.frame_width)
    cap.set(cv2.CAP_PROP_FRAME_HEIGHT, config.frame_height)

    print("System Started. Press 'q' to quit.")
    print("【モード】画面全体を5x4のグリッドとして判定します")

    try:
        while True:
            ret, frame = cap.read()
            if not ret: break

            frame = process_frame(frame, detector, judge, manager)
            cv2.imshow("Kyudo Board Reader (Full Screen)", frame)

            if cv2.waitKey(1) & 0xFF == ord('q'):
                break

    finally:
        cap.release()
        cv2.destroyAllWindows()
        manager.close()

if __name__ == "__main__":
    main()