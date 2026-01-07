import numpy as np
import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d import Axes3D

# 1-ノルム（マンハッタン距離）の単位球
def plot_l1_norm(ax, r=1, color='r', alpha=0.3):
    verts = np.array([
        [ 1, 0, 0],
        [-1, 0, 0],
        [ 0, 1, 0],
        [ 0,-1, 0],
        [ 0, 0, 1],
        [ 0, 0,-1]
    ]) * r

    faces = [
        [verts[0], verts[2], verts[4]],
        [verts[0], verts[2], verts[5]],
        [verts[0], verts[3], verts[4]],
        [verts[0], verts[3], verts[5]],
        [verts[1], verts[2], verts[4]],
        [verts[1], verts[2], verts[5]],
        [verts[1], verts[3], verts[4]],
        [verts[1], verts[3], verts[5]]
    ]

    from mpl_toolkits.mplot3d.art3d import Poly3DCollection
    poly3d = Poly3DCollection(faces, alpha=alpha, facecolor=color)
    ax.add_collection3d(poly3d)

# 2-ノルム（ユークリッド距離）の単位球
def plot_l2_norm(ax, r=1, color='b', alpha=0.3):
    u = np.linspace(0, 2*np.pi, 50)
    v = np.linspace(0, np.pi, 50)
    x = r * np.outer(np.cos(u), np.sin(v))
    y = r * np.outer(np.sin(u), np.sin(v))
    z = r * np.outer(np.ones(np.size(u)), np.cos(v))
    ax.plot_surface(x, y, z, color=color, alpha=alpha, edgecolor='none')

# ∞-ノルム（チェビシェフ距離）の単位球
def plot_linf_norm(ax, r=1, color='g', alpha=0.3):
    r = r / 2**0
    for s in [-1, 1]:
        ax.plot_surface(s*r*np.ones((2, 2)), np.array([[-r, -r],[r, r]]), np.array([[-r, r],[-r, r]]), color=color, alpha=alpha)
        ax.plot_surface(np.array([[-r, -r],[r, r]]), s*r*np.ones((2, 2)), np.array([[-r, r],[-r, r]]), color=color, alpha=alpha)
        ax.plot_surface(np.array([[-r, -r],[r, r]]), np.array([[-r, r],[-r, r]]), s*r*np.ones((2, 2)), color=color, alpha=alpha)

fig = plt.figure(figsize=(10, 8))
ax = fig.add_subplot(111, projection='3d')
ax.set_box_aspect([1, 1, 1])

plot_l1_norm(ax, r=1, color='red', alpha=0.4)
plot_l2_norm(ax, r=1, color='blue', alpha=0.4)
plot_linf_norm(ax, r=1, color='green', alpha=0.4)

ax.set_xlim([-1.5, 1.5])
ax.set_ylim([-1.5, 1.5])
ax.set_zlim([-1.5, 1.5])
ax.set_xlabel('X')
ax.set_ylabel('Y')
ax.set_zlabel('Z')
ax.set_title('3D 1-norm, 2-norm, ∞_norm')
plt.show()