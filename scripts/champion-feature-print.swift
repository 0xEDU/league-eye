import Foundation
import Vision

func featurePrint(path: String) throws -> VNFeaturePrintObservation {
    let request = VNGenerateImageFeaturePrintRequest()
    try VNImageRequestHandler(url: URL(fileURLWithPath: path), options: [:]).perform([request])
    guard let result = request.results?.first as? VNFeaturePrintObservation else {
        throw NSError(domain: "league-vision", code: 1)
    }
    return result
}

let arguments = CommandLine.arguments
guard arguments.count == 3 else {
    fputs("usage: champion-feature-print QUERY REFERENCE_DIRECTORY\n", stderr)
    exit(2)
}

let query = try featurePrint(path: arguments[1])
let directory = URL(fileURLWithPath: arguments[2])
let files = try FileManager.default.contentsOfDirectory(
    at: directory,
    includingPropertiesForKeys: nil
).filter { $0.pathExtension == "png" }
var results: [(String, Float)] = []
for file in files {
    let candidate = try featurePrint(path: file.path)
    var distance: Float = 0
    try query.computeDistance(&distance, to: candidate)
    results.append((file.deletingPathExtension().lastPathComponent, distance))
}
for result in results.sorted(by: { $0.1 < $1.1 }).prefix(10) {
    print("\(result.0)\t\(result.1)")
}
