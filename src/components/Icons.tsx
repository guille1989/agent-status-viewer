import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement>;

const shared = {
  width: 18,
  height: 18,
  viewBox: "0 0 24 24",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.8,
  strokeLinecap: "round" as const,
  strokeLinejoin: "round" as const,
  "aria-hidden": true,
};

export function CheckIcon(props: IconProps) {
  return <svg {...shared} {...props}><circle cx="12" cy="12" r="9" /><path d="m8.5 12 2.25 2.25L15.8 9.2" /></svg>;
}

export function AlertIcon(props: IconProps) {
  return <svg {...shared} {...props}><path d="M10.4 4.1 2.7 17.5A1.7 1.7 0 0 0 4.2 20h15.6a1.7 1.7 0 0 0 1.5-2.5L13.6 4.1a1.8 1.8 0 0 0-3.2 0Z" /><path d="M12 9v4" /><path d="M12 16.5h.01" /></svg>;
}

export function LoaderIcon(props: IconProps) {
  return <svg {...shared} {...props}><path d="M20 12a8 8 0 1 1-2.34-5.66" /><path d="M20 4v6h-6" /></svg>;
}

export function PlugIcon(props: IconProps) {
  return <svg {...shared} {...props}><path d="M8 12h8" /><path d="M9 8V4" /><path d="M15 8V4" /><path d="M18 8v3a6 6 0 0 1-12 0V8Z" /><path d="M12 17v3" /></svg>;
}

export function TicketIcon(props: IconProps) {
  return <svg {...shared} {...props}><path d="M6 3h12v3a2 2 0 0 0 0 4v3a2 2 0 0 0 0 4v4H6v-4a2 2 0 0 0 0-4v-3a2 2 0 0 0 0-4Z" /><path d="M10 8h4" /><path d="M10 12h4" /><path d="M10 16h3" /></svg>;
}
