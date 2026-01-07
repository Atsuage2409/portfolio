import cv2
from ultralytics import YOLO

model = YOLO('best.pt')

def main():
    cap = cv2.VideoCapture(2)
    
    cap.set(cv2.CAP_PROP_FRAME_WIDTH, 1920)
    cap.set(cv2.CAP_PROP_FRAME_HEIGHT, 1080)

    print("デバッグモード開始。'q'キーで終了します。")

    while True:
        ret, frame = cap.read()
        if not ret:
            break
    
        if frame.shape[2] == 4:
            frame = cv2.cvtColor(frame, cv2.COLOR_BGRA2BGR)
        results = model(frame, conf=0.65)
        annotated_frame = results[0].plot()
        print(results)
        cv2.imshow("YOLO Debug Monitor", annotated_frame)

        if cv2.waitKey(1) & 0xFF == ord('q'):
            break
    cap.release()
    cv2.destroyAllWindows()

if __name__ == "__main__":
    main()