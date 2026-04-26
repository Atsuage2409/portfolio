import config
import math

class GridJudge:
    def __init__(self, min_stable_frames=3):
        """
        min_stable_frames: 表示を更新するために必要な連続一致フレーム数
        瞬間的な遮蔽（人が前を通るなど）では更新されない
        """
        self.stable_scoreboard = [["-" for _ in range(config.num_targets)]
                                  for _ in range(config.num_shots)]
        self.candidate_scoreboard = None
        self.candidate_count = 0
        self.min_stable_frames = min_stable_frames

    def _boards_equal(self, a, b):
        return all(a[i][j] == b[i][j]
                   for i in range(config.num_shots)
                   for j in range(config.num_targets))

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
            new_scoreboard = scoreboard
        else:
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

            new_scoreboard = scoreboard

        # 安定フレーム判定: min_stable_frames 連続で同じ結果なら表示を更新
        if self.candidate_scoreboard is not None and self._boards_equal(self.candidate_scoreboard, new_scoreboard):
            self.candidate_count += 1
        else:
            self.candidate_scoreboard = new_scoreboard
            self.candidate_count = 1

        if self.candidate_count >= self.min_stable_frames:
            self.stable_scoreboard = new_scoreboard

        return self.stable_scoreboard
