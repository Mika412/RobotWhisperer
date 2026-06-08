import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import type { ServerResponse } from "node:http";

import type { Plugin } from "vite";
import type { Accessor, Buffer as GltfBuffer, Document } from "@gltf-transform/core";
import type { BufferAttribute, InterleavedBufferAttribute, Mesh, Object3D } from "three";

type Toolkit = {
  three: typeof import("three");
  io: import("@gltf-transform/core").NodeIO;
  draco: typeof import("@gltf-transform/functions").draco;
  Document: typeof import("@gltf-transform/core").Document;
  mergeVertices: typeof import("three/examples/jsm/utils/BufferGeometryUtils.js").mergeVertices;
  loaders: {
    collada: import("three/examples/jsm/loaders/ColladaLoader.js").ColladaLoader;
    obj: import("three/examples/jsm/loaders/OBJLoader.js").OBJLoader;
    stl: import("three/examples/jsm/loaders/STLLoader.js").STLLoader;
  };
};

type MeshPart = { geometry: import("three").BufferGeometry; color: number[] };

const MESH_EXTENSIONS = [".dae", ".obj", ".stl"];
const CACHE_DIRECTORY = path.resolve("node_modules/.cache/rw-mesh");
const DRACO_SOURCE = path.resolve("node_modules/three/examples/jsm/libs/draco");
const TRANSCODE_VERSION = "2";
const QUANTIZATION = { quantizePosition: 16, quantizeNormal: 10, quantizeGeneric: 12 };

const CONTENT_TYPES: Record<string, string> = {
  ".glb": "model/gltf-binary",
  ".wasm": "application/wasm",
  ".js": "text/javascript",
};

function createTranscoder(): (sourcePath: string) => Promise<Buffer> {
  let toolkit: Toolkit | null = null;

  async function toolkitOnce(): Promise<Toolkit> {
    if (toolkit) return toolkit;
    const [
      three,
      { ColladaLoader },
      { OBJLoader },
      { STLLoader },
      { mergeVertices },
      { DOMParser },
      { Document, NodeIO },
      { KHRDracoMeshCompression },
      { draco },
      draco3d,
    ] = await Promise.all([
      import("three"),
      import("three/examples/jsm/loaders/ColladaLoader.js"),
      import("three/examples/jsm/loaders/OBJLoader.js"),
      import("three/examples/jsm/loaders/STLLoader.js"),
      import("three/examples/jsm/utils/BufferGeometryUtils.js"),
      import("@xmldom/xmldom"),
      import("@gltf-transform/core"),
      import("@gltf-transform/extensions"),
      import("@gltf-transform/functions"),
      import("draco3dgltf"),
    ]);

    globalThis.DOMParser ??= DOMParser as unknown as typeof globalThis.DOMParser;

    const io = new NodeIO()
      .registerExtensions([KHRDracoMeshCompression])
      .registerDependencies({ "draco3d.encoder": await draco3d.createEncoderModule() });

    toolkit = {
      three,
      io,
      draco,
      Document,
      mergeVertices,
      loaders: { collada: new ColladaLoader(), obj: new OBJLoader(), stl: new STLLoader() },
    };
    return toolkit;
  }

  function parseSource(kit: Toolkit, sourcePath: string): Object3D {
    const extension = path.extname(sourcePath).toLowerCase();
    if (extension === ".stl") {
      const geometry = kit.loaders.stl.parse(toArrayBuffer(fs.readFileSync(sourcePath)));
      return new kit.three.Mesh(geometry);
    }
    const text = fs.readFileSync(sourcePath, "utf-8");
    if (extension === ".obj") return kit.loaders.obj.parse(text);
    const collada = kit.loaders.collada.parse(text, path.dirname(sourcePath));
    return collada ? collada.scene : new kit.three.Object3D();
  }

  function extractParts(kit: Toolkit, root: Object3D): MeshPart[] {
    root.position.set(0, 0, 0);
    root.quaternion.identity();
    root.updateMatrixWorld(true);
    const parts: MeshPart[] = [];
    root.traverse((child) => {
      const mesh = child as Mesh;
      if (!mesh.isMesh || !mesh.geometry) return;
      let geometry = mesh.geometry.clone();
      geometry.applyMatrix4(mesh.matrixWorld);
      if (!geometry.getAttribute("normal")) geometry.computeVertexNormals();
      for (const name of Object.keys(geometry.attributes))
        if (name !== "position" && name !== "normal") geometry.deleteAttribute(name);
      if (!geometry.index) geometry = kit.mergeVertices(geometry);
      const material = Array.isArray(mesh.material) ? mesh.material[0] : mesh.material;
      const colored = (material ?? {}) as { color?: import("three").Color };
      parts.push({ geometry, color: colored.color ? colored.color.toArray() : [1, 1, 1] });
    });
    return parts;
  }

  function buildDocument(kit: Toolkit, parts: MeshPart[]): Document {
    const document = new kit.Document();
    const buffer = document.createBuffer();
    const scene = document.createScene();
    for (const { geometry, color } of parts) {
      const position = geometry.getAttribute("position");
      const normal = geometry.getAttribute("normal");
      const index = geometry.index;
      if (!index) continue;
      const primitive = document
        .createPrimitive()
        .setAttribute("POSITION", vectorAccessor(document, buffer, position))
        .setIndices(indexAccessor(document, buffer, index.array, position.count));
      if (normal) primitive.setAttribute("NORMAL", vectorAccessor(document, buffer, normal));
      primitive.setMaterial(
        document
          .createMaterial()
          .setBaseColorFactor([color[0], color[1], color[2], 1])
          .setMetallicFactor(0)
          .setRoughnessFactor(1),
      );
      scene.addChild(document.createNode().setMesh(document.createMesh().addPrimitive(primitive)));
    }
    return document;
  }

  return async function transcode(sourcePath: string): Promise<Buffer> {
    const kit = await toolkitOnce();
    const parts = extractParts(kit, parseSource(kit, sourcePath));
    const document = buildDocument(kit, parts);
    await document.transform(kit.draco(QUANTIZATION));
    return Buffer.from(await kit.io.writeBinary(document));
  };
}

function vectorAccessor(
  document: Document,
  buffer: GltfBuffer,
  attribute: BufferAttribute | InterleavedBufferAttribute,
): Accessor {
  const values = new Float32Array(attribute.count * 3);
  for (let index = 0; index < attribute.count; index += 1) {
    values[index * 3] = attribute.getX(index);
    values[index * 3 + 1] = attribute.getY(index);
    values[index * 3 + 2] = attribute.getZ(index);
  }
  return document.createAccessor().setType("VEC3").setBuffer(buffer).setArray(values);
}

function indexAccessor(
  document: Document,
  buffer: GltfBuffer,
  source: ArrayLike<number>,
  vertexCount: number,
): Accessor {
  const Indices = vertexCount > 65535 ? Uint32Array : Uint16Array;
  const array = source instanceof Indices ? source : Indices.from(source);
  return document.createAccessor().setType("SCALAR").setBuffer(buffer).setArray(array);
}

function toArrayBuffer(nodeBuffer: Buffer): ArrayBuffer {
  return nodeBuffer.buffer.slice(
    nodeBuffer.byteOffset,
    nodeBuffer.byteOffset + nodeBuffer.byteLength,
  ) as ArrayBuffer;
}

function listMeshes(directory: string): string[] {
  const meshes: string[] = [];
  for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
    const entryPath = path.join(directory, entry.name);
    if (entry.isDirectory()) meshes.push(...listMeshes(entryPath));
    else if (MESH_EXTENSIONS.includes(path.extname(entry.name).toLowerCase()))
      meshes.push(entryPath);
  }
  return meshes;
}

async function cachedGlb(
  transcode: (sourcePath: string) => Promise<Buffer>,
  sourcePath: string,
): Promise<Buffer> {
  const source = fs.readFileSync(sourcePath);
  const hash = crypto
    .createHash("sha256")
    .update(TRANSCODE_VERSION)
    .update(JSON.stringify(QUANTIZATION))
    .update(source)
    .digest("hex");
  const cachePath = path.join(CACHE_DIRECTORY, `${hash}.glb`);
  if (fs.existsSync(cachePath)) return fs.readFileSync(cachePath);
  const glb = await transcode(sourcePath);
  fs.mkdirSync(CACHE_DIRECTORY, { recursive: true });
  fs.writeFileSync(cachePath, glb);
  return glb;
}

function sendBuffer(response: ServerResponse, buffer: Buffer, nameOrPath: string): void {
  response.setHeader(
    "Content-Type",
    CONTENT_TYPES[path.extname(nameOrPath)] ?? "application/octet-stream",
  );
  response.end(buffer);
}

export default function meshOptimize(): Plugin {
  const assetsDir = path.resolve("static/assets");
  const transcode = createTranscoder();
  let base = "/";
  let outDir = "";

  function findSource(relativeGlb: string): string | null {
    const withoutExtension = relativeGlb.slice(0, -".glb".length);
    for (const extension of MESH_EXTENSIONS) {
      const candidate = path.join(assetsDir, `${withoutExtension}${extension}`);
      if (fs.existsSync(candidate)) return candidate;
    }
    return null;
  }

  return {
    name: "mesh-optimize",

    configResolved(config) {
      base = config.base ?? "/";
      outDir = config.build.outDir;
    },

    configureServer(server) {
      server.middlewares.use(async (request, response, next) => {
        const url = decodeURIComponent((request.url ?? "").split("?")[0]);
        const prefix = base.endsWith("/") ? base : `${base}/`;
        const relative = url.startsWith(prefix) ? url.slice(prefix.length) : url.replace(/^\//, "");

        if (relative.startsWith("draco/")) {
          const file = path.join(DRACO_SOURCE, relative.slice("draco/".length));
          if (fs.existsSync(file)) return sendBuffer(response, fs.readFileSync(file), file);
          return next();
        }

        if (!(relative.startsWith("assets/") && relative.endsWith(".glb"))) return next();
        const source = findSource(relative.slice("assets/".length));
        if (!source) return next();
        try {
          sendBuffer(response, await cachedGlb(transcode, source), relative);
        } catch (error) {
          next(error);
        }
      });
    },

    async closeBundle() {
      const outputAssets = path.join(outDir, "assets");
      if (!fs.existsSync(outputAssets)) return;

      for (const source of listMeshes(outputAssets)) {
        const glbPath = `${source.slice(0, -path.extname(source).length)}.glb`;
        fs.writeFileSync(glbPath, await cachedGlb(transcode, source));
        fs.rmSync(source);
      }

      fs.cpSync(DRACO_SOURCE, path.join(outDir, "draco"), { recursive: true });
    },
  };
}
