struct Vector3 {
  float x, y, z;
};

vec3 to_vec3(Vector3 v) { return vec3(v.x, v.y, v.z); }

float max_component(Vector3 v) {
  return max(abs(v.x), max(abs(v.y), abs(v.z)));
}

struct AABB {
  Vector3 minPos;
  Vector3 maxPos;
};

struct Quaternion {
  float i, j, k, r;
};

vec4 quat_to_vec4(Quaternion q) { return vec4(q.i, q.j, q.k, q.r); }

mat3 quat_to_mat3(Quaternion q) {
  return mat3(vec3(1 - 2 * (q.j * q.j + q.k * q.k), 2 * (q.i * q.j + q.k * q.r),
                   2 * (q.i * q.k - q.j * q.r)),
              vec3(2 * (q.i * q.j - q.k * q.r), 1 - 2 * (q.i * q.i + q.k * q.k),
                   2 * (q.j * q.k + q.i * q.r)),
              vec3(2 * (q.i * q.k + q.j * q.r), 2 * (q.j * q.k - q.i * q.r),
                   1 - 2 * (q.i * q.i + q.j * q.j)));
}

mat4 quat_to_mat4(Quaternion q) {
  return mat4(vec4(1 - 2 * (q.j * q.j + q.k * q.k), 2 * (q.i * q.j + q.k * q.r),
                   2 * (q.i * q.k - q.j * q.r), 0),
              vec4(2 * (q.i * q.j - q.k * q.r), 1 - 2 * (q.i * q.i + q.k * q.k),
                   2 * (q.j * q.k + q.i * q.r), 0),
              vec4(2 * (q.i * q.k + q.j * q.r), 2 * (q.j * q.k - q.i * q.r),
                   1 - 2 * (q.i * q.i + q.j * q.j), 0),
              vec4(0, 0, 0, 1));
}

struct Transform {
  Vector3 translation;
  Vector3 scale;
  Quaternion rotation;
};

struct Mesh {
  uint firstIndex;
  uint indexCount;
  int vertexOffset;
};

struct Sphere {
  Vector3 center;
  float radius;
};

struct Model {
  Sphere bounds;
  Mesh mesh;
};

struct Object {
  Transform transform;
  uint model;
};

struct DrawCommand {
  uint indexCount;
  uint instanceCount;
  uint firstIndex;
  int vertexOffset;
  uint firstInstance;
};

struct DrawData {
  mat4 transform;
  vec4 color;
};
