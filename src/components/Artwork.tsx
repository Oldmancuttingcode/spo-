import { useState } from "react";
import "./Artwork.css";

export default function Artwork({ url, className = "" }: { url: string | null; className?: string }) {
  const [failedUrl, setFailedUrl] = useState<string | null>(null);
  return url && failedUrl !== url ? (
    <img className={`artwork ${className}`} src={url} alt="" loading="lazy" onError={() => setFailedUrl(url)} />
  ) : (
    <span className={`artwork artwork-placeholder ${className}`} role="img" aria-label="No artwork">♪</span>
  );
}
