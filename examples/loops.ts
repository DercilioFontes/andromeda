console.log("Testing loops...");

const N = 5000; // 5000 iterations triggers the issue

// Test 1: Plain for loop
let plainLoopActual = -1;

for (let i = 0; i < N; i++) {
  plainLoopActual = i + 1;
}

console.assert(plainLoopActual === N, "Plain for loop failed");
console.log("✅ Plain for loop passed.");

// Test 2: Map with primitive value
const primitiveValueMap = new Map<number, number>();

for (let i = 0; i < N; i++) {
  primitiveValueMap.set(i, i);
}

console.assert(primitiveValueMap.size === N, "Primitive value map for loop failed");
console.log("✅ Primitive value map for loop passed.");

// Test 3: Map with object value
class Cell {
  x: number;

  constructor(x: number) {
    this.x = x;
  }
}

const objectValueMap = new Map<number, Cell>();

for (let x = 0; x < N; x++) {
  // console.count("loop") // console inside the loop triggers the issue with less iterations
  const cell = new Cell(x);
  objectValueMap.set(x, cell);
}

console.assert(objectValueMap.size === N, "Object value map for loop failed");
console.log("✅ Object value map for loop passed.");

console.log("Testing loops completed successfully.");
