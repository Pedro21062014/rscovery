import { Link, useLocation } from "react-router-dom";

const scanOptions = [
  {
    name: "🖼️ JPEG",
    route: "/images?type=jpeg&id=",
  },
  {
    name: "🖼️ PNG",
    route: "/images?type=png&id=",
  },
  {
    name: "📄 PDF",
    route: "/file?type=pdf&id=",
  },
  {
    name: "📦 ZIP",
    route: "/file?type=zip&id=",
  },
  {
    name: "📄 Text",
    route: "/text?id=",
  },
  {
    name: "📹 MP4",
    route: "/file?type=mp4&id=",
  },
  {
    name: "🔍 View Blocks",
    route: "/blocks?id=",
    big: true,
  },
];

export default function Scanning() {
  const { search } = useLocation();
  const queryParams = new URLSearchParams(search);

  const id = queryParams.get("id");

  return (
    <main className="container">
      <header>
        <div>
          <Link to={"/"}>Go Back</Link>
        </div>
        <h1>Disk "{id}"</h1>
      </header>

      <div className="options">
        {scanOptions.map(({ name, route, big }) => (
          <div key={name} style={big ? { gridColumn: "span 2" } : {}}>
            <Link to={`${route}${id}`}>
              <div>{name}</div>
            </Link>
          </div>
        ))}
      </div>
    </main>
  );
}
