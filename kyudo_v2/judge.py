import config
import math

class GridJudge:
    def update(self, raw_detections):
        """
        scoreboard[行][列]
        行: 0=一番上 → 下
        列: 0=一番左 → 右
        下から詰めて埋める
        """

        #初期化
        scoreboard = [["-" for _ in range(config.num_targets)]
                      for _ in range(config.num_shots)]

        #中心座標を求める
        points = []
        for det in raw_detections:
            x1, y1, x2, y2 = det['box']
            cx = (x1 + x2) // 2
            cy = (y1 + y2) // 2
            points.append({
                'cx': cx,
                'cy': cy,
                'name': det['name']
            })
        if not points:
            return scoreboard

        #Y座標で下から上にソート
        points.sort(key=lambda p: p['cy'], reverse=True)

        for i in range(config.num_shots):
            src_start = i * config.num_targets
            src_end = src_start + config.num_targets

            row_points = points[src_start:src_end]
            # X座標で右から左にソート
            row_points.sort(key=lambda p: p['cx'], reverse=True)
            row = config.num_shots - i - 1
            for n,p in enumerate(row_points):
                name = p['name']
                col = config.num_targets - n - 1
                if name in ["O", "maru", "circle"]:
                    mark = "◯"
                elif name in ["X", "batsu", "cross"]:
                    mark = "✕"
                else:
                    mark = "-"

                scoreboard[row][col] = mark

        return scoreboard
