/**
 * Analyzes an object and returns the serialized representation with type information
 * @param obj - The object to analyze
 * @returns Object containing field names, value types, and serialized bytes
 */
export function schema_struct_serialize(obj: any): {
    fieldNames: string[];
    valueTypes: number[];
    byteLayout: Uint8Array;
  } {
    const fieldNames = Object.keys(obj);
    const valueTypes: number[] = [];
    const valueBytes: Uint8Array[] = [];
  
    // Process each value in the object
    Object.values(obj).forEach(value => {
      let typeId: number;
      let bytes: Uint8Array;
  
      if (typeof value === 'string') {
        typeId = 12; // String
        bytes = new Uint8Array(Buffer.from(value, 'utf8'));
      } else if (typeof value === 'number') {
        if (Number.isInteger(value)) {
          if (value >= 0) {
            if (value <= 255) {
              typeId = 0; // u8
              bytes = new Uint8Array([value]);
            } else if (value <= 65535) {
              typeId = 1; // u16
              const buffer = Buffer.alloc(2);
              buffer.writeUInt16LE(value);
              bytes = new Uint8Array(buffer);
            } else if (value <= 4294967295) {
              typeId = 2; // u32
              const buffer = Buffer.alloc(4);
              buffer.writeUInt32LE(value);
              bytes = new Uint8Array(buffer);
            } else {
              typeId = 3; // u64
              const buffer = Buffer.alloc(8);
              buffer.writeBigUInt64LE(BigInt(value));
              bytes = new Uint8Array(buffer);
            }
          } else {
            // Handle negative integers
            if (value >= -128) {
              typeId = 5; // i8
              bytes = new Uint8Array([value & 0xFF]);
            } else if (value >= -32768) {
              typeId = 6; // i16
              const buffer = Buffer.alloc(2);
              buffer.writeInt16LE(value);
              bytes = new Uint8Array(buffer);
            } else if (value >= -2147483648) {
              typeId = 7; // i32
              const buffer = Buffer.alloc(4);
              buffer.writeInt32LE(value);
              bytes = new Uint8Array(buffer);
            } else {
              typeId = 8; // i64
              const buffer = Buffer.alloc(8);
              buffer.writeBigInt64LE(BigInt(value));
              bytes = new Uint8Array(buffer);
            }
          }
        } else {
          // Handle floating point numbers as u64 (you might want to adjust this)
          typeId = 3; // u64
          const buffer = Buffer.alloc(8);
          buffer.writeDoubleLE(value);
          bytes = new Uint8Array(buffer);
        }
      } else if (typeof value === 'boolean') {
        typeId = 10; // bool
        bytes = new Uint8Array([value ? 1 : 0]);
      } else if (Array.isArray(value)) {
        // Handle arrays - determine type from first element
        if (value.length === 0) {
          typeId = 13; // Vec<u8> as default
          bytes = new Uint8Array(0);
        } else {
          const firstElement = value[0];
          if (typeof firstElement === 'string') {
            typeId = 25; // Vec<String>
          } else if (typeof firstElement === 'number') {
            typeId = 13; // Vec<u8> (you might want to be more specific)
          } else {
            typeId = 13; // Vec<u8> as default
          }
          bytes = new Uint8Array(Buffer.from(JSON.stringify(value), 'utf8'));
        }
      } else if (typeof value === 'object' && value !== null) {
        typeId = 12; // String (serialize as JSON)
        bytes = new Uint8Array(Buffer.from(JSON.stringify(value), 'utf8'));
      } else {
        typeId = 0; // u8 as default
        bytes = new Uint8Array(0);
      }
  
      valueTypes.push(typeId);
      valueBytes.push(bytes);
    });
  
    // Calculate total length and concatenate all byte arrays
    const totalLength = valueBytes.reduce((sum, bytes) => sum + bytes.length, 0);
    const byteLayout = new Uint8Array(totalLength);
    let offset = 0;
    for (const bytes of valueBytes) {
      byteLayout.set(bytes, offset);
      offset += bytes.length;
    }
  
    return {
      fieldNames,
      valueTypes,
      byteLayout
    };
  }