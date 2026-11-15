/**
 * The brand symbol without its plate: three lines reaching one interchange station. The brand guide allows it only
 * on the signal frame (a dark ground), which is the only place it is used. Decoration next to the app name.
 */
export function BrandMark() {
  return (
    <svg className="brand-mark" viewBox="0 0 1024 1024" width="28" height="28" aria-hidden="true" focusable="false">
      <path className="brand-line-1" d="M0 192h320v128H0zM704 192h320v128H704z" />
      <path className="brand-line-2" d="M0 448h320v128H0zM704 448h320v128H704z" />
      <path className="brand-line-4" d="M0 704h320v128H0zM704 704h320v128H704z" />
      <rect className="brand-station" x="384" y="64" width="256" height="896" rx="128" />
    </svg>
  );
}
