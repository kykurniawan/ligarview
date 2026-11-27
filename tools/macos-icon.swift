// Composes the macOS app icon from the full-bleed artwork: macOS icons keep the shape inside an 824x824
// body centred in a 1024x1024 canvas, with a soft shadow below. Without that margin the icon looks
// larger than every other app's in the Dock and the Cmd+Tab switcher.
//
//   swift tools/macos-icon.swift assets/ligarview-icon.png assets/ligarview-icon-mac.png
import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

let args = CommandLine.arguments
guard args.count == 3,
      let source = CGImageSourceCreateWithURL(URL(fileURLWithPath: args[1]) as CFURL, nil),
      let art = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
    FileHandle.standardError.write(Data("usage: macos-icon.swift in.png out.png\n".utf8))
    exit(1)
}

let size = 1024, body = 824
let ctx = CGContext(data: nil, width: size, height: size, bitsPerComponent: 8, bytesPerRow: size * 4,
                    space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!

// Crop to the opaque artwork first: the source has a few pixels of margin on one side only.
let W = art.width, H = art.height
let probe = CGContext(data: nil, width: W, height: H, bitsPerComponent: 8, bytesPerRow: W * 4,
                      space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
probe.draw(art, in: CGRect(x: 0, y: 0, width: W, height: H))
let px = probe.data!.assumingMemoryBound(to: UInt8.self)
var (minX, maxX, minY, maxY) = (W, 0, H, 0)
for y in 0..<H { for x in 0..<W where px[(y * W + x) * 4 + 3] > 128 {
    minX = min(minX, x); maxX = max(maxX, x); minY = min(minY, y); maxY = max(maxY, y)
} }
let crop = art.cropping(to: CGRect(x: minX, y: minY, width: maxX - minX + 1, height: maxY - minY + 1))!

// Fit the artwork into the body square, keeping its proportions, centred.
let scale = min(CGFloat(body) / CGFloat(crop.width), CGFloat(body) / CGFloat(crop.height))
let w = CGFloat(crop.width) * scale, h = CGFloat(crop.height) * scale
let rect = CGRect(x: (CGFloat(size) - w) / 2, y: (CGFloat(size) - h) / 2, width: w, height: h)
ctx.setShadow(offset: CGSize(width: 0, height: -12), blur: 24, color: CGColor(gray: 0, alpha: 0.3))
ctx.interpolationQuality = .high
ctx.draw(crop, in: rect)

let dest = CGImageDestinationCreateWithURL(URL(fileURLWithPath: args[2]) as CFURL, UTType.png.identifier as CFString, 1, nil)!
CGImageDestinationAddImage(dest, ctx.makeImage()!, nil)
guard CGImageDestinationFinalize(dest) else { exit(1) }
print("Wrote \(args[2]) (artwork \(Int(w))x\(Int(h)) in a \(size)x\(size) canvas)")
