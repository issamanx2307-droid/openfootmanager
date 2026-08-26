import { useState, type ReactNode } from "react";

interface AssetImageProps {
  src: string | null;
  alt: string;
  className?: string;
  fallback: ReactNode;
}

export default function AssetImage({
  src,
  alt,
  className = "",
  fallback,
}: AssetImageProps) {
  const [failedSrc, setFailedSrc] = useState<string | null>(null);
  const failed = failedSrc === src;

  if (!src || failed) {
    return <>{fallback}</>;
  }

  return (
    <img
      src={src}
      alt={alt}
      className={className}
      loading="lazy"
      referrerPolicy="no-referrer"
      onError={() => setFailedSrc(src)}
    />
  );
}
