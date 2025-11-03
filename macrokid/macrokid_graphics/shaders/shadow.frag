#version 450

// For shadow mapping, we only need depth values
// No fragment output needed - depth buffer handles everything

void main() {
    // Fragment shader can be empty for basic shadow mapping
    // Depth testing and writing is handled automatically
    // 
    // If we needed alpha testing or other effects, we could:
    // - Sample textures for alpha cutoff
    // - Discard fragments conditionally
    // - Output custom depth values
}