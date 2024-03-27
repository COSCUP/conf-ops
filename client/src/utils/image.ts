
export function getImageSize (file: File): Promise<{ width: number, height: number }> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = function (e) {
      const img = new Image()
      img.onload = function () {
        resolve({ width: img.width, height: img.height })
      }
      img.onerror = reject
      img.src = e.target?.result as string
    }
    reader.onerror = reject
    reader.readAsDataURL(file)
  })
}
