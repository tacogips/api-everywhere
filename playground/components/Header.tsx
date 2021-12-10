import * as React from "react";
import GithubCorner from "react-github-corner";
import { useState, useEffect } from "react";

type Props = {};

const Header: React.FC<Props> = () => {
  return (
    <>
      <nav className="flex items-center justify-between flex-wrap bg-teal-500 ">
        <div className="flex items-center flex-shrink-0 text-white mr-6 p-6">
          <h1 className="font-semibold text-xl tracking-tight">
            API Everywhere
          </h1>
        </div>
        <div>
          <GithubCorner href="https://github.com/tacogips/api-everywhere" />
        </div>
      </nav>
    </>
  );
};

export default Header;
