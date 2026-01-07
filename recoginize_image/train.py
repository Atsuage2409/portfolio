from ultralytics import YOLO

if __name__ == '__main__':
    
    model = YOLO('yolov8m.pt') 

    results = model.train(
        data='./data_set/data.yaml',
        epochs=1000,
        imgsz=640,
        device=0,
        batch=128,  
        name='kyudo_model',
        workers=20  
    )