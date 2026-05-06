import { AlertTriangle } from "lucide-react";
import { Link } from "react-router-dom";

export default function NotFound() {
  return (
    <div className="text-center py-20">
      <AlertTriangle size={48} className="mx-auto text-gray-600 mb-4" />
      <h1 className="text-xl font-bold text-gray-300 mb-2">Page not found</h1>
      <p className="text-gray-400 mb-6">
        The page you’re looking for doesn’t exist or has been moved.
      </p>
      <Link to="/" className="btn btn-safe inline-flex items-center gap-2">
        Back home
      </Link>
    </div>
  );
}
