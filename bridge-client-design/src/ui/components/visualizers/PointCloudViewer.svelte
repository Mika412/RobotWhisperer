<script lang="ts">
    import { Canvas, useFrame } from "threlte";
    import * as THREE from "three";
    import { latest } from "../../store";
    export let resourceName: string;
    let positions = new Float32Array();
    $: msg = $latest[resourceName]?.data;
    $: if (msg && msg.points) {
        // Assume msg.points: Float32Array [x,y,z,...]
        positions = new Float32Array(msg.points);
    }
</script>

<Canvas class="h-[480px] w-full border">
    <scene>
        <perspectiveCamera position={[0, 0, 5]} />
        <orbitControls />
        <points>
            <bufferGeometry>
                <bufferAttribute
                    attach="attributes-position"
                    array={positions}
                    itemSize={3}
                />
            </bufferGeometry>
            <pointsMaterial size={0.02} />
        </points>
        <ambientLight intensity={0.5} />
        <directionalLight position={[2, 2, 2]} />
    </scene>
</Canvas>
