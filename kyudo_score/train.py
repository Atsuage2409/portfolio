from ultralytics import YOLO

if __name__ == '__main__':
    
    # 1. ベースモデルの読み込み
    model = YOLO('yolov8m.pt') 

    # 2. 学習開始
    results = model.train(
        data='./data_set/data.yaml',
        epochs=1000,
        imgsz=640,
        device=0,
        batch=128,  
        name='kyudo_model',
        workers=20  
    )